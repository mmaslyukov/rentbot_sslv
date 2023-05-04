use chrono;
use derive_builder::Builder;
use regex::Regex;
use reqwest::header::CONTENT_TYPE;
use scraper::{Html, Selector};
use std::{collections::HashMap, fmt::Display};
#[derive(Debug)]
enum SSError {
    Index(usize),
    ConvertChar(u32),
    Http(String),
    Selector(String),
    Parse(String),
    Empty,
}

impl std::error::Error for SSError {}

impl Display for SSError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

#[derive(Debug, Clone)]
struct Location {
    longitude: f64,
    latitude: f64,
}

#[derive(Default, Debug, Builder)]
struct Apartment {
    id: String,
    date: String, // TODO: Changle to chrono date
    city: String,
    district: String,
    address: String,
    location: Option<Location>,
    rooms: u64,
    area: f64,
    lift: bool,
    partking: Option<String>,
    school_distance: Option<f64>,
}
// POST Requests Arguments
static PA_PRICE_LOW: &str = "topt[8][min]";
static PA_PRICE_HIGH: &str = "topt[8][max]";
static PA_AREA_LOW: &str = "topt[3][min]";

fn decode(g: &'static str, r: &'static str, k: u64) -> Result<String, Box<dyn std::error::Error>> {
    let base = base64::decode(g.as_bytes())?;
    let data_url = std::str::from_utf8(base.as_slice())?;
    // println!("Decoded: {}", data);
    let data_bytes = urlencoding::decode_binary(data_url.as_bytes()).into_owned();
    let data_len = data_bytes.len();
    let key_bytes = r.as_bytes();
    let key_len = key_bytes.len();
    let mut plain = String::new();
    for i in 0..data_len {
        let dbyte = data_bytes.get(i).ok_or(SSError::Index(i))?;
        let kbyte = key_bytes.get(i % key_len).ok_or(SSError::Index(i))?;
        let rbyte: u32 = (*dbyte as i32 - *kbyte as i32 + 14) as u32;
        plain = format!(
            "{}{}",
            plain,
            std::char::from_u32(rbyte as u32).ok_or(SSError::ConvertChar(rbyte))?
        );
    }
    println!("res: {}", plain);
    Ok((plain))
}

struct ApartmentPage {
    id: String,
    page: Html,
    apartment_details: Apartment,
}

impl ApartmentPage {
    fn new(id: String, page: Html) -> Self {
        Self {
            id,
            page,
            apartment_details: Apartment::default(),
        }
    }
    fn parse_attr(
        &self,
        selector_str: &str,
        attr: &str,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let selector = Selector::parse(&selector_str).unwrap();
        let value = self
            .page
            .select(&selector)
            .next()
            .ok_or(Box::new(SSError::Selector(selector_str.to_string())))?;
        // println!("{:?}", value);
        let attr_value = value
            .value()
            .attr(attr)
            .ok_or(Box::new(SSError::Selector(selector_str.to_string())))?
            .to_string();
        Ok(attr_value)
    }
    fn parse_string(&self, selector_str: &str) -> Result<String, Box<dyn std::error::Error>> {
        // println!("{:?}", selector_str);
        let selector = Selector::parse(&selector_str).unwrap();
        let value = self
            .page
            .select(&selector)
            .next()
            .ok_or(Box::new(SSError::Selector(selector_str.to_string())))?
            .inner_html();
        // println!("{:?}", value);
        Ok(value)
    }

    fn parse_f64(&self, selector_str: &str) -> Result<f64, Box<dyn std::error::Error>> {
        let selector = Selector::parse(&selector_str).unwrap();
        let value = self
            .page
            .select(&selector)
            .next()
            .ok_or(Box::new(SSError::Selector(selector_str.to_string())))?
            .inner_html();
        // println!("{:?}", value);
        let area = value.split(' ').next().unwrap().parse()?;
        Ok(area)
    }

    fn parse_price(&self) -> Result<String, Box<dyn std::error::Error>> {
        self.parse_string("#tdo_8")
    }

    fn parse_area(&self) -> Result<f64, Box<dyn std::error::Error>> {
        self.parse_f64("#tdo_3")
    }

    fn parse_rooms(&self) -> Result<f64, Box<dyn std::error::Error>> {
        self.parse_f64("#tdo_1")
    }

    fn parse_parking(&self) -> Result<bool, Box<dyn std::error::Error>> {
        let parking = self.parse_string("#tdo_1734")?.to_lowercase();
        let found = Regex::new(r#"парков"#)?.is_match(&parking);
        println!("p: {}", found);
        Ok(found)
    }
    fn parse_description(&self) -> Result<(bool, bool), Box<dyn std::error::Error>> {
        let descr = self.parse_string("#msg_div_msg")?.to_lowercase();
        let parking_found = Regex::new(r#"парков|parki"#)?.is_match(&descr);
        let lift_found = Regex::new(r#"лифт|lift|elevator"#)?.is_match(&descr);
        println!("p: {}, l: {}", parking_found, lift_found);
        Ok((parking_found, lift_found))
        // #msg_div_msg
    }

    fn parse_floor(&self) -> Result<(u64, bool), Box<dyn std::error::Error>> {
        let floor_line = self.parse_string("#tdo_4")?.to_lowercase();
        let elevator_found = Regex::new(r#"лифт"#)?.is_match(&floor_line);
        let floor = floor_line
            .split('/')
            .next()
            .ok_or(Box::new(SSError::Parse(format!(
                "Fail to parse floor out of {}",
                floor_line
            ))))?
            .parse()?;

        Ok((floor, elevator_found))
    }
    fn parse_location(&self) -> Result<Location, Box<dyn std::error::Error>> {
        let map = self.parse_attr("#mnu_map", "onclick")?.to_lowercase();
        // Get coordinates out of 'onclick' attribute
        let re = Regex::new(r#"&c=([[:digit:]]+\.[[:digit:]]+), ([[:digit:]]+\.[[:digit:]]+)"#)?;
        let captures = re.captures(&map).ok_or(Box::new(SSError::Parse(format!(
            "Fail to parse coordinates out of {}",
            map
        ))))?;
        let lat = captures
            .get(1)
            .ok_or(Box::new(SSError::Parse(
                "Fail to parse latitude".to_string(),
            )))?
            .as_str()
            .parse()?;
        let lon = captures
            .get(2)
            .ok_or(Box::new(SSError::Parse(
                "Fail to parse longitude".to_string(),
            )))?
            .as_str()
            .parse()?;

        // println!("{:?}", captures);
        Ok(Location {
            longitude: lon,
            latitude: lat,
        })
    }

    fn parse_city(&self) -> Result<String, Box<dyn std::error::Error>> {
        let city = self.parse_string("#tdo_20")?;
        Ok(city)
    }
    fn parse_district(&self) -> Result<String, Box<dyn std::error::Error>> {
        let district = self.parse_string("#tdo_856")?;
        Ok(district)
    }
    fn parse_address(&self) -> Result<String, Box<dyn std::error::Error>> {
        let address = self.parse_string("#tdo_11")?;
        Ok(address)
    }
    fn parse_datetime(&self) -> Result<String, Box<dyn std::error::Error>> {
        let datetime = self.parse_string("td.msg_footer:nth-child(2)")?;
        let re = Regex::new(
            "([[:digit:]]+\\.[[:digit:]]+\\.[[:digit:]]+ [[:digit:]]+:[[:digit:]]+:[[:digit:]]+)",
        )?
        .captures(datetime.as_str())
        .ok_or(Box::new(SSError::Parse(format!(
            "Fail to parse datetime: {}",
            datetime
        ))))?
        .get(1)
        .ok_or(Box::new(SSError::Parse(format!(
            "Fail to parse datetime: {}",
            datetime
        ))))?;
        println!("{:?}", datetime);
        // chrono::NaiveDate::parse_from_str(s, fmt)
        Ok(String::new())
    }

    fn parse(&mut self) -> Result<Apartment, Box<dyn std::error::Error>> {
        // println!("{:?}", self.page);
        let city = self.parse_city()?;
        // return Ok(());
        let district = self.parse_district()?;
        let address = self.parse_address()?;
        let price = self.parse_price()?;
        let area = self.parse_area()?;
        let rooms = self.parse_rooms()?;
        let parking = self.parse_parking()?;
        let (descr_parking, desct_elevator) = self.parse_description()?;
        let (floor, floor_elevator) = self.parse_floor()?;
        println!(
            "city: {}\ndistrict: {}\naddress: {}\nprice: {}\nrooms: {}\narea: {} \nfloor: {}\nparking: {}\nelevator: {}\n",
            city,
            district,
            address,
            price,
            rooms,
            area,
            floor,
            parking || descr_parking,
            desct_elevator || floor_elevator,
        );
        let loc = self.parse_location()?;
        let datetime = self.parse_datetime()?;
        println!("location: {:?}", loc);
        // let apartment = ApartmentBuilder::default().id(self.id).
        Ok(ApartmentBuilder::default().build()?)
    }
}
// struct SearchPage {
//     url: String,
//     appartments: Vec<ApartmentPage>,
// }

// impl SearchPage {}
#[derive(Default)]
struct SearchPageBuilder<'a> {
    url: &'a str,
    args: HashMap<&'a str, u32>,
}
impl<'a> SearchPageBuilder<'a> {
    fn new() -> Self {
        Self::default()
    }
    fn url(mut self, url: &'a str) -> Self {
        self.url = url;
        self
    }
    fn min_price(mut self, price: u32) -> Self {
        *self.args.entry(PA_PRICE_LOW).or_insert(price) = price;
        self
    }

    fn max_price(mut self, price: u32) -> Self {
        *self.args.entry(PA_PRICE_HIGH).or_insert(price) = price;
        self
    }

    fn min_area(mut self, area: u32) -> Self {
        *self.args.entry(PA_AREA_LOW).or_insert(area) = area;
        self
    }

    fn build(self) -> Result<SearchPageRequest, Box<dyn std::error::Error>> {
        Ok(SearchPageRequest {
            url: reqwest::Url::parse(self.url)?,
            body: serde_urlencoded::to_string(&self.args)?,
        })
    }
}

struct SearchPageRequest {
    url: reqwest::Url, //&'a str,
    body: String,
}
impl SearchPageRequest {
    fn request(
        self,
        client: &reqwest::blocking::Client,
    ) -> Result<SearchPage, Box<dyn std::error::Error>> {
        // let client = reqwest::blocking::ClientBuilder::new()
        //     .cookie_store(true)
        //     .build()
        //     .unwrap();

        let request_post = client
            .post(self.url.clone())
            .header(reqwest::header::CONTENT_LENGTH, self.body.len())
            .header(reqwest::header::CONNECTION, "keep-alive")
            .header(reqwest::header::ACCEPT, "*/*")
            // .header(reqwest::header::ACCEPT_ENCODING, "gzip, deflate, br")
            .header(
                reqwest::header::CONTENT_TYPE,
                "application/x-www-form-urlencoded",
            )
            .header(reqwest::header::USER_AGENT, "agent")
            .body(self.body.clone())
            .build()?;

        let response = client.execute(request_post.try_clone().unwrap())?;
        println!(
            "POST: status:{} headers:{:?}",
            response.status(),
            response.headers()
        );
        let text = response.text().unwrap();
        println!("page size: {} KB", text.len() as f64 / 1000.0);
        if (true) {
            let request_get = client
                .get(self.url.clone())
                .header(reqwest::header::CONNECTION, "keep-alive")
                .header(reqwest::header::ACCEPT, "*/*")
                .header(reqwest::header::USER_AGENT, "agent")
                .build()?;
            // } else {
            // let request_post = client
            //     .post(self.url)
            //     .header(reqwest::header::CONTENT_LENGTH, self.body.len())
            //     .header(reqwest::header::CONNECTION, "keep-alive")
            //     .header(reqwest::header::ACCEPT, "*/*")
            //     // .header(reqwest::header::ACCEPT_ENCODING, "gzip, deflate, br")
            //     .header(
            //         reqwest::header::CONTENT_TYPE,
            //         "application/x-www-form-urlencoded",
            //     )
            //     .header(reqwest::header::USER_AGENT, "agent")
            //     .body(self.body)
            //     .build()?;
        }
        let response = client.execute(request_post.try_clone().unwrap())?;
        println!(
            "GET/POST: status:{} headers:{:?}",
            response.status(),
            response.headers()
        );
        let text = response.text().unwrap();
        println!("page size: {} KB", text.len() as f64 / 1000.0);
        // println!("{}", text);
        Ok(SearchPage::new(
            self.url,
            scraper::Html::parse_document(text.as_str()),
        ))
        // Err(Box::new(SSError::Http(self.url.to_string())))
    }
}

#[derive(Debug)]
struct ApartmentPageRequest {
    id: String,
    href: String,
}
impl ApartmentPageRequest {
    // fn new(id: String, url: String) -> Self {
    //     Self {
    //         id,
    //         url: reqwest::Url::parse(url).unwrap(),
    //     }
    // }
    fn request(
        &self,
        client: &reqwest::blocking::Client,
    ) -> Result<ApartmentPage, Box<dyn std::error::Error>> {
        let url = reqwest::Url::parse(self.href.as_str())?;
        let response = client.get(url.clone()).send()?;
        if 200 != response.status() {
            return Err(Box::new(SSError::Http(url.to_string())));
        }
        let body = response.text()?;
        // response.status().eq(reqwest::Response::St)
        println!("Got page size: {} KB", body.len() as f64 / 1000.0);
        // println!("len: {:?}", body);
        // Err(Box::new(SSError::Empty))
        Ok(ApartmentPage::new(
            self.id.clone(),
            Html::parse_document(&body),
        ))
    }
}

struct SearchPage {
    url: reqwest::Url,
    page: Html,
    row_id: u32,
    apartments: Vec<ApartmentPageRequest>,
}

impl SearchPage {
    fn new(url: reqwest::Url, html: Html) -> Self {
        Self {
            url,
            page: html,
            row_id: 2,
            apartments: Vec::new(),
        }
    }

    fn next(&mut self) -> Result<ApartmentPageRequest, Box<dyn std::error::Error>> {
        Ok(self.apartments.pop().ok_or(Box::new(SSError::Empty))?)
    }

    fn parse(mut self) -> Result<Self, Box<dyn std::error::Error>> {
        let selector_path = "#filter_frm > table:nth-child(3) > tbody:nth-child(1)";
        println!("Use selector: '{}'", selector_path);
        let selector = Selector::parse(&selector_path).unwrap();
        let attr_name = "href";
        let app = self
            .page
            .select(&selector)
            .next()
            .ok_or(Box::new(SSError::Selector(selector_path.to_string())))?;
        let search_results = app.children().filter_map(|a| match a.value().as_element() {
            Some(el) if el.attr("style") == None => match el.attr("id") {
                Some(id) if id != "head_line" => Some(a),
                _ => None,
            },
            _ => None,
        });

        self.apartments = search_results
            .clone()
            .map(|l| {
                let id = l.value().as_element().unwrap().attr("id").unwrap();
                (id, l)
            })
            .filter_map(|t| match t.1.children().next() {
                Some(n) => Some((t.0, n)),
                _ => None,
            })
            .filter_map(|n| match n.1.next_sibling() {
                Some(s) => Some((n.0, s)),
                _ => None,
            })
            .filter_map(|s| match s.1.first_child() {
                Some(c) if c.value().is_element() => Some((s.0, c.value().as_element().unwrap())),
                _ => None,
            })
            .filter_map(|c| match c.1.attr(attr_name) {
                Some(href) => {
                    // println!("hreg: {:?}", href);
                    Some(ApartmentPageRequest {
                        id: c.0.to_string(),
                        href: format!(
                            "{}://{}{}",
                            self.url.scheme(),
                            self.url.host_str().unwrap(),
                            href.to_string()
                        ),
                    })
                }
                _ => None,
            })
            .collect();
        println!("Found {} appartments", self.apartments.len());
        Ok(self)
    }
}

async fn request_page(url: &str) -> Result<Html, Box<dyn std::error::Error>> {
    let resp = reqwest::get(url).await?;
    // let resp = reqwest::blocking::get(url)?;
    println!("Request url({}): '{}'", resp.status(), url);
    if resp.status().as_u16() != 200 {
        Err(Box::new(SSError::Http(resp.status().as_str().to_string())))
    } else {
        let body = resp.text().await?;
        Ok(Html::parse_document(&body))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    //*[@id="contacts_js"]

    // %8B%A2wTt%8E%7D%5Cv%A2wWq%7D%8F%94z%7Dg%8Fry%94%98s%7D%8DTmy%93%5C

    //<script async="" id="contacts_js" src="/js/ru/2023-03-27/6c92445aa0b94c2cee79f62a56ad8946ab403ff5d10abe33fc3d38cda40d9db5.js?d=%2FzN0xMdRYerwH%2FfH7J7TvWnbBhiKyUltnn%2B0T9AX3gs58DI7SaAwL85haA1shC57Ujem0oadnw2GfHWzHBZY0tkiplbZsihbyMKcojrD6DOa9wEx9HAYNtgpaJOaGNzc0p5d4NW8GTMtQx2H5%2FgyUUVi5PNql9xwiPPldHAi8B7QpQx%2BxeLf%2Fd6GY6Abw08iaWuOdCOm8l17hgg9Z0MkPVNmzXNVU3314TzUTXUk0ty5BoiqDNRLGnAoLHcVCYj%2FNMluVcA9bHRVf%2BvRUj57uUXfSPQSs0QtpQbTqsW2097F7l2b%2BnmRcfLd9CgFroE8yz9zqa5PrhpCQ5M4JK8jGwjakm5fD4yQxpNuSSoeI%2BsPy2uRnXZWQhSwYt6Qbba7G9miXPrHpf%2BdiiEPZaJ56I7K9L88nbN8U7TPkY4pNEyfnsDwjkwbOxW0MisCEriIThP%2FtZRFqCgEGXNT7ZSvFQ%3D%3D&amp;c=1"></script>

    // 1|JTg5JTlDJTdCVnYlOTIlN0JXdSVBMHhacyU4RSU3RVpwJTdCJTdEZXMlOEV6JTVFdXolOTMlOTNyeiU5MCU5MXh6eGU=|85730698	2|JTg3JTk2WCU4RXlqbyU5MHlqJTdEWWwlN0UlOTFabiU3RSU5Mm8=|82688600

    // key=key*6-47289+517
    // -----------------------
    // let phase1 = decode(
    //     "JThCJUEyd1R0JThFJTdEJTVDdiVBMndXcSU3RCU4RiU5NHolN0RnJThGcnklOTQlOThzJTdEJThEVG15JTkzJTVD",
    //     "77011366",
    //     2,
    // );
    // decode("byU1QiU4MyU4NXglQTElOTlpJTk1JTk4", "K0dbVwzGrpLa-wRs2", 2);

    let client = reqwest::blocking::ClientBuilder::new()
        .cookie_store(true)
        .build()
        .unwrap();
    let mut sp = SearchPageBuilder::new()
        .url("https://www.ss.lv/ru/real-estate/flats/riga/today-2/hand_over/filter/")
        .min_price(1000)
        .build()?
        .request(&client)?
        .parse()?;

    while let Ok(apartment_page_request) = sp.next() {
        println!(
            "{} => {}",
            apartment_page_request.id, apartment_page_request.href
        );
        apartment_page_request.request(&client)?.parse()?;
        break;
    }
    return Ok(());
}
