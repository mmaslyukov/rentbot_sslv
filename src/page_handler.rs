use std::{collections::HashMap, sync::Arc};

use crate::{
    apartment::{Apartment, ApartmentBuilder, ApartmentDescription, Location},
    error::SSError,
};
use regex::Regex;
use scraper::{Html, Selector};

// POST Requests Arguments
static PA_PRICE_LOW: &str = "topt[8][min]";
static PA_PRICE_HIGH: &str = "topt[8][max]";
static PA_AREA_LOW: &str = "topt[3][min]";
const EARTH_RADIUS: f64 = 6_371_000 as f64;
const TARGET_LOCATION: Location = Location {
    latitude: 56.9585757,
    longitude: 24.1257553,
};

pub struct ApartmentPage {
    pub url: String,
    pub id: String,
    pub page: Html,
    // apartment_details: Apartment,
}

impl ApartmentPage {
    pub fn new(url: String, id: String, page: Html) -> Self {
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

    pub fn parse_price(&self) -> Result<String, Box<dyn std::error::Error>> {
        self.parse_string("#tdo_8")
    }

    pub fn parse_area(&self) -> Result<f64, Box<dyn std::error::Error>> {
        self.parse_f64("#tdo_3")
    }

    pub fn parse_rooms(&self) -> Result<f64, Box<dyn std::error::Error>> {
        self.parse_f64("#tdo_1")
    }

    pub fn parse_parking(&self) -> Result<bool, Box<dyn std::error::Error>> {
        let parking = self.parse_string("#tdo_1734")?.to_lowercase();
        let found = Regex::new(r#"парков"#)?.is_match(&parking);
        Ok(found)
    }
    pub fn parse_description_p_e(
        &self,
    ) -> Result<ApartmentDescription, Box<dyn std::error::Error>> {
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

    pub fn parse_floor_f_e(&self) -> Result<(i64, bool), Box<dyn std::error::Error>> {
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
    pub fn parse_location(&self) -> Result<Location, Box<dyn std::error::Error>> {
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

    pub fn parse_city(&self) -> Result<String, Box<dyn std::error::Error>> {
        let city = self.parse_string("#tdo_20 > b")?;
        Ok(city)
    }
    pub fn parse_district(&self) -> Result<String, Box<dyn std::error::Error>> {
        let district = self.parse_string("#tdo_856 > b")?;
        Ok(district)
    }
    pub fn parse_address(&self) -> Result<String, Box<dyn std::error::Error>> {
        let address = self.parse_string("#tdo_11 > b")?;
        Ok(address)
    }
    pub fn parse_datetime(&self) -> Result<chrono::NaiveDateTime, Box<dyn std::error::Error>> {
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

    pub fn parse(self) -> Result<Apartment, Box<dyn std::error::Error>> {
        // println!("{:?}", self.page);
        let city = self.parse_city().unwrap_or_default();
        // return Ok(());
        let district = self.parse_district().unwrap_or_default();
        let address = self.parse_address().unwrap_or_default();
        let price = self.parse_price().unwrap_or_default();
        let area = self.parse_area().unwrap_or_default();
        let rooms = self.parse_rooms().unwrap_or_default() as u64;
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
pub struct SearchPageBuilder<'a> {
    pub url: &'a str,
    pub args: HashMap<&'a str, u32>,
}
impl<'a> SearchPageBuilder<'a> {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn url(mut self, url: &'a str) -> Self {
        self.url = url;
        self
    }
    pub fn min_price(mut self, price: u32) -> Self {
        *self.args.entry(PA_PRICE_LOW).or_insert(price) = price;
        self
    }

    pub fn max_price(mut self, price: u32) -> Self {
        *self.args.entry(PA_PRICE_HIGH).or_insert(price) = price;
        self
    }

    pub fn min_area(mut self, area: u32) -> Self {
        *self.args.entry(PA_AREA_LOW).or_insert(area) = area;
        self
    }

    pub fn build(self) -> Result<SearchPageRequest, Box<dyn std::error::Error>> {
        Ok(SearchPageRequest {
            url: reqwest::Url::parse(self.url)?,
            body: serde_urlencoded::to_string(&self.args)?,
        })
    }
}

pub struct SearchPageRequest {
    pub url: reqwest::Url, //&'a str,
    pub body: String,
}
impl SearchPageRequest {
    pub async fn request(
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
        log::debug!(
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
        log::debug!(
            "2nd POST: status:{} headers:{:?}",
            response.status(),
            response.headers()
        );
        let text = response.text().await?;
        log::info!("Page size: {} KB", text.len() as f64 / 1000.0);
        // println!("{}", text);
        Ok(SearchPage::new(
            self.url,
            scraper::Html::parse_document(text.as_str()),
        ))
        // Err(Box::new(SSError::Http(self.url.to_string())))
    }
}

#[derive(Debug)]
pub struct ApartmentPageRequest {
    pub id: String,
    pub href: String,
}
impl ApartmentPageRequest {
    // fn new(id: String, url: String) -> Self {
    //     Self {
    //         id,
    //         url: reqwest::Url::parse(url).unwrap(),
    //     }
    // }
    pub async fn request(
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
        log::info!(
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

pub struct SearchPage {
    pub url: reqwest::Url,
    pub page: Html,
    pub apartments: Vec<ApartmentPageRequest>,
}

impl SearchPage {
    pub fn new(url: reqwest::Url, html: Html) -> Self {
        Self {
            url,
            page: html,
            apartments: Vec::new(),
        }
    }

    pub fn next(&mut self) -> Result<ApartmentPageRequest, Box<dyn std::error::Error>> {
        Ok(self.apartments.pop().ok_or(Box::new(SSError::Empty))?)
    }

    pub fn parse(mut self) -> Result<Self, Box<dyn std::error::Error>> {
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
        log::info!("Found {} appartments", self.apartments.len());
        Ok(self)
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
