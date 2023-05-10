use chrono;
use derive_builder::Builder;
use regex::Regex;
use scraper::{Html, Selector};
use std::{
    collections::{hash_map::IterMut, HashMap},
    fmt::Display,
    sync::Arc,
};
use teloxide::prelude::*;
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
    latitude: f64,
    longitude: f64,
}

#[derive(Debug, Clone, Default)]
struct ApartmentDescription {
    park: bool,
    elevator: bool,
    balkony: bool,
}

impl ApartmentDescription {
    fn new(park: bool, elevator: bool, balkony: bool) -> Self {
        Self {
            park,
            balkony,
            elevator,
        }
    }
}

#[derive(Default, Debug, Builder, Clone)]
struct Apartment {
    url: String,
    id: String,
    price: String,
    datetime: chrono::NaiveDateTime, // TODO: Changle to chrono date
    city: String,
    district: String,
    address: String,
    #[builder(default)]
    location: Option<Location>,
    #[builder(default)]
    distance: Option<i64>,
    rooms: u64,
    area: f64,
    floor: Option<i64>,
    elevator: bool,
    parking: bool,
    #[builder(default)]
    description: Option<ApartmentDescription>,
}

const EARTH_RADIUS: f64 = 6_371_000 as f64;
const TARGET_LOCATION: Location = Location {
    latitude: 56.9585757,
    longitude: 24.1257553,
};

// POST Requests Arguments
static PA_PRICE_LOW: &str = "topt[8][min]";
static PA_PRICE_HIGH: &str = "topt[8][max]";
static PA_AREA_LOW: &str = "topt[3][min]";

fn _decode(
    g: &'static str,
    r: &'static str,
    _k: u64,
) -> Result<String, Box<dyn std::error::Error>> {
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
    Ok(plain)
}

struct ApartmentPage {
    url: String,
    id: String,
    page: Html,
    // apartment_details: Apartment,
}

impl ApartmentPage {
    fn new(url: String, id: String, page: Html) -> Self {
        Self {
            url,
            id,
            page,
            // apartment_details: Apartment::default(),
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
        Ok(found)
    }
    fn parse_description_p_e(&self) -> Result<ApartmentDescription, Box<dyn std::error::Error>> {
        let descr = self.parse_string("#msg_div_msg")?.to_lowercase();
        let park_found = Regex::new(r#" парк| park"#)?.is_match(&descr);
        let balkony_found = Regex::new(r#" балкон| терасса| balkony| terrace"#)?.is_match(&descr);
        let elevator_found = Regex::new(r#" лифт| lift| elevator"#)?.is_match(&descr);
        Ok(ApartmentDescription::new(
            park_found,
            elevator_found,
            balkony_found,
        ))
        // #msg_div_msg
    }

    fn parse_floor_f_e(&self) -> Result<(i64, bool), Box<dyn std::error::Error>> {
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
        let city = self.parse_string("#tdo_20 > b")?;
        Ok(city)
    }
    fn parse_district(&self) -> Result<String, Box<dyn std::error::Error>> {
        let district = self.parse_string("#tdo_856 > b")?;
        Ok(district)
    }
    fn parse_address(&self) -> Result<String, Box<dyn std::error::Error>> {
        let address = self.parse_string("#tdo_11 > b")?;
        Ok(address)
    }
    fn parse_datetime(&self) -> Result<chrono::NaiveDateTime, Box<dyn std::error::Error>> {
        let datetime_parsed = self.parse_string("td.msg_footer:nth-child(2)")?;
        let datetime_str =
            Regex::new("([[:digit:]]+\\.[[:digit:]]+\\.[[:digit:]]+ [[:digit:]]+:[[:digit:]]+)")?
                .captures(datetime_parsed.as_str())
                .ok_or(Box::new(SSError::Parse(format!(
                    "Fail to parse datetime: {}",
                    datetime_parsed
                ))))?
                .get(1)
                .ok_or(Box::new(SSError::Parse(format!(
                    "Fail to get match of datetime: {}",
                    datetime_parsed
                ))))?
                .as_str();
        // println!("{:?}", datetime_str);
        Ok(chrono::NaiveDateTime::parse_from_str(
            datetime_str,
            "%d.%m.%Y %H:%M",
        )?)
    }

    fn parse(self) -> Result<Apartment, Box<dyn std::error::Error>> {
        // println!("{:?}", self.page);
        let city = self.parse_city()?;
        // return Ok(());
        let district = self.parse_district()?;
        let address = self.parse_address()?;
        let price = self.parse_price()?;
        let area = self.parse_area()?;
        let rooms = self.parse_rooms()? as u64;
        let parking = self.parse_parking().unwrap_or(false);
        let descr = self.parse_description_p_e().ok();
        let floor = self.parse_floor_f_e().ok();
        // println!(
        //     "city: {}\ndistrict: {}\naddress: {}\nprice: {}\nrooms: {}\narea: {} \nfloor: {:?}\nparking: {:?}",
        //     city,
        //     district,
        //     address,
        //     price,
        //     rooms,
        //     area,
        //     floor,
        //     parking,
        // );
        let loc = self.parse_location().ok();
        let datetime = self.parse_datetime()?;
        // println!("datetime: {:?}", datetime);
        // println!("location: {:?}", loc);
        // // let apartment = ApartmentBuilder::default().id(self.id).
        let mut floor_elevator = false;
        let mut floor_number = None;
        if let Some(floor) = floor {
            floor_elevator = floor.1;
            floor_number = Some(floor.0);
        }
        let distance = if let Some(l) = loc.as_ref() {
            Some(calculate_distance(
                l.latitude,
                l.longitude,
                TARGET_LOCATION.latitude,
                TARGET_LOCATION.longitude,
            ) as i64)
        } else {
            None
        };
        Ok(ApartmentBuilder::default()
            .url(self.url)
            .id(self.id)
            .datetime(datetime)
            .city(city)
            .district(district)
            .address(address)
            .rooms(rooms)
            .parking(parking)
            .elevator(floor_elevator)
            .price(price)
            .area(area)
            .floor(floor_number)
            .location(loc)
            .distance(distance)
            .description(descr)
            .build()?)
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
    async fn request(
        self,
        client: &reqwest::Client,
    ) -> Result<SearchPage, Box<dyn std::error::Error>> {
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

        let response = client.execute(request_post.try_clone().unwrap()).await?;
        println!(
            "1st POST: status:{} headers:{:?}",
            response.status(),
            response.headers()
        );
        let _text = response.text().await?;
        // println!("page size: {} KB", text.len() as f64 / 1000.0);
        if false {
            let _request_get = client
                .get(self.url.clone())
                .header(reqwest::header::CONNECTION, "keep-alive")
                .header(reqwest::header::ACCEPT, "*/*")
                .header(reqwest::header::USER_AGENT, "agent")
                .build()?;
        }
        let response = client.execute(request_post.try_clone().unwrap()).await?;
        println!(
            "2nd POST: status:{} headers:{:?}",
            response.status(),
            response.headers()
        );
        let text = response.text().await?;
        println!("Page size: {} KB", text.len() as f64 / 1000.0);
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
    async fn request(
        &self,
        client: Arc<reqwest::Client>,
    ) -> Result<ApartmentPage, Box<dyn std::error::Error + Send + Sync>> {
        let url = reqwest::Url::parse(self.href.as_str())?;
        let response = client.get(url.clone()).send().await?;
        if 200 != response.status() {
            return Err(Box::new(SSError::Http(url.to_string())));
        }
        let body = response.text().await?;
        // response.status().eq(reqwest::Response::St)
        println!(
            "Got page: id({}) size({} KB), ulr({})",
            self.id,
            body.len() as f64 / 1000.0,
            self.href
        );
        // println!("len: {:?}", body);
        // Err(Box::new(SSError::Empty))
        Ok(ApartmentPage::new(
            self.href.clone(),
            self.id.clone(),
            Html::parse_document(&body),
        ))
    }
}

struct SearchPage {
    url: reqwest::Url,
    page: Html,
    apartments: Vec<ApartmentPageRequest>,
}

impl SearchPage {
    fn new(url: reqwest::Url, html: Html) -> Self {
        Self {
            url,
            page: html,
            apartments: Vec::new(),
        }
    }

    fn next(&mut self) -> Result<ApartmentPageRequest, Box<dyn std::error::Error>> {
        Ok(self.apartments.pop().ok_or(Box::new(SSError::Empty))?)
    }

    fn parse(mut self) -> Result<Self, Box<dyn std::error::Error>> {
        let selector_path = "#filter_frm > table:nth-child(3) > tbody:nth-child(1)";
        // println!("Use selector: '{}'", selector_path);
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

#[derive(Default, PartialEq)]
enum ApartmentLifeCycle {
    #[default]
    Active,
    Sent,
}

// impl ApartmentLifeCycle {
//     fn phase_send(&mut self) {
//         match self {
//             Self::Active => *self = Self::Sent,
//             _ => {}
//         }
//     }
// }

#[derive(Default)]
struct ApartmentWrapper {
    apartment: Apartment,
    lifecycle: ApartmentLifeCycle,
    expired: bool,
}
impl From<Apartment> for ApartmentWrapper {
    fn from(value: Apartment) -> Self {
        Self {
            apartment: value,
            ..Default::default()
        }
    }
}
#[derive(Default)]
struct ApartmentCache {
    apartments: HashMap<String, ApartmentWrapper>,
}

impl ApartmentCache {
    fn update(&mut self, apartments: Vec<Apartment>) {
        for a in apartments {
            let key = a.id.clone();
            // println!("cache size is {}", self.apartments.len());
            if !self.apartments.contains_key(&key) {
                println!("Insert a key:{}", key);
                self.apartments.insert(key.clone(), a.into());
            } else {
                let value = self.apartments.get_mut(&key).unwrap();
                value.expired = false;
            }
            // println!("cache size is {}", self.apartments.len());
        }
        // Remove expired entires
        self.apartments.retain(|_, v| !v.expired);
    }

    fn iter(&mut self) -> IterMut<'_, String, ApartmentWrapper> {
        self.apartments.iter_mut()
    }
}

// function to calculate the distance between two points
fn calculate_distance(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    let delta_lat = (lat2 - lat1).to_radians();
    let delta_lon = (lon2 - lon1).to_radians();

    let a = (delta_lat / 2.0).sin().powi(2)
        + lat1.to_radians().cos() * lat2.to_radians().cos() * (delta_lon / 2.0).sin().powi(2);
    let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());

    EARTH_RADIUS * c
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

    // let client = reqwest::blocking::ClientBuilder::new()
    let client = Arc::new(
        reqwest::ClientBuilder::new()
            .cookie_store(true)
            .build()
            .unwrap(),
    );

    let bot = Bot::from_env();
    let mut cache = ApartmentCache::default();
    let chat_id = std::env::var("TELOXIDE_CHAT_ID")?;
    println!("token => {}, chat id: {}", bot.token(), chat_id);
    loop {
        let mut sp = SearchPageBuilder::new()
            .url("https://www.ss.lv/ru/real-estate/flats/riga/today-2/hand_over/filter/")
            .min_area(70)
            .max_price(1000)
            .min_price(500)
            .build()?
            .request(&client)
            .await?
            .parse()?;

        let mut handles = vec![];
        while let Ok(apartment_page_request) = sp.next() {
            handles.push(tokio::spawn(handle_page(
                apartment_page_request,
                client.clone(),
            )));
        }

        let apartments: Vec<Apartment> = futures::future::join_all(handles)
            .await
            .iter()
            .map(|j| j.as_ref().unwrap().as_ref().unwrap().clone())
            .collect();

        cache.update(apartments);

        // println!("cache size is {} before the loop", cache.apartments.len());
        for entry in cache.iter() {
            let a = &entry.1.apartment;
            let mut skip = false;
            if entry.1.lifecycle == ApartmentLifeCycle::Sent {
                skip = true;
            }
            if !a.elevator
                && a.description.is_some()
                && !a.description.as_ref().unwrap().elevator
                && a.floor.is_some()
                && *a.floor.as_ref().unwrap() != 1
            {
                skip = true;
            }
            if !skip {
                println!("Sending new apartment: id({}), url({})", a.id, a.url);
                let send_status = bot
                .send_message(
                    chat_id.clone(),
                    format!(
                        "дата:{}\nцена:{}, комн:{}, пл.:{} м2, дист:{} м, этаж:{}, лифт:{}, п.м.:{}, \nоп(л:{}, п:{}, б:{}) \nссылка: {}",
                        a.datetime,
                        a.price,
                        a.rooms,
                        a.area,
                        a.distance.unwrap_or(-1),
                        a.floor.unwrap_or_default(),
                        if a.elevator { "+" } else { "-" },
                        if a.parking { "+" } else { "-" },
                        if a.description.as_ref().unwrap_or(&ApartmentDescription::default()).elevator { "+" } else { "-" },
                        if a.description.as_ref().unwrap_or(&ApartmentDescription::default()).park{ "+" } else { "-" },
                        if a.description.as_ref().unwrap_or(&ApartmentDescription::default()).balkony{ "+" } else { "-" },
                        a.url
                    ),
                )
                .await;
                if send_status.is_ok() {
                    entry.1.lifecycle = ApartmentLifeCycle::Sent;
                }
            }

            entry.1.expired = true;
            // println!("send");
        }
        // println!("cache size is {} after the loop", cache.apartments.len());
        // println!("sleep");
        tokio::time::sleep(tokio::time::Duration::from_secs(60 * 15)).await;
    }
    // return Ok(());
}

async fn handle_page(apr: ApartmentPageRequest, client: Arc<reqwest::Client>) -> Option<Apartment> {
    apr.request(client).await.unwrap().parse().ok()
}
