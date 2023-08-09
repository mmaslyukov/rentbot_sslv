use rentbot_sslv::{
    apartment::*,
    config::Config,
    db::record::ApartmentRecrod,
    error::SSError,
    page_handler::{ApartmentPageRequest, SearchPageBuilder},
};
use std::{
    collections::{hash_map::IterMut, HashMap},
    sync::Arc,
};
use teloxide::prelude::*;

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

#[derive(Default, PartialEq, Copy, Clone, Debug)]
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
    fn update(&mut self, apartments: Vec<Option<Apartment>>) {
        for a in apartments {
            match a {
                Some(a) => {
                    let key = a.id.clone();
                    // println!("cache size is {}", self.apartments.len());
                    if !self.apartments.contains_key(&key) {
                        log::info!("Insert a key:{}", key);
                        self.apartments.insert(key.clone(), a.into());
                    } else {
                        let value = self.apartments.get_mut(&key).unwrap();
                        value.expired = false;
                    }
                    // println!("cache size is {}", self.apartments.len());
                }
                None => {}
            }
        }
        // Remove expired entires
        self.apartments.retain(|_, v| !v.expired);
    }

    fn iter(&mut self) -> IterMut<'_, String, ApartmentWrapper> {
        self.apartments.iter_mut()
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

    // let client = reqwest::blocking::ClientBuilder::new()

    // simple_logging::log_to_file("rentsslv.log", log::LevelFilter::Trace)?;
    pretty_env_logger::formatted_timed_builder()
        .filter(Some("rentbot_sslv"), log::LevelFilter::Trace)
        .init();
    let client = Arc::new(
        reqwest::ClientBuilder::new()
            .cookie_store(true)
            .build()
            .unwrap(),
    );
    // log::debug!("test");
    let bot = Bot::from_env();
    let mut cache = ApartmentCache::default();
    let chat_id_opt = std::env::var("TELOXIDE_CHAT_ID").ok();
    log::info!("token => {}, chat id: {:?}", bot.token(), chat_id_opt);
    if chat_id_opt.is_some() {
        bot.send_message(chat_id_opt.clone().unwrap(), "--==| Rebooted |==--")
            .await
            .unwrap();
    }
    loop {
        let mut sp = SearchPageBuilder::new()
            .url("https://www.ss.lv/ru/real-estate/flats/riga/today-2/hand_over/filter/")
            .min_area(Config::area_low())
            .max_price(Config::price_high())
            .min_price(Config::price_low())
            .build()?
            .request(&client)
            .await?
            .parse()?;

        let mut handlers = vec![];
        // println!("{:?}", sp.apartments);
        while let Ok(apartment_page_request) = sp.next() {
            handlers.push(tokio::spawn(handle_page(
                apartment_page_request,
                client.clone(),
            )));
        }

        let apartments: Vec<Option<Apartment>> = futures::future::join_all(handlers)
            .await
            .iter()
            .map(|j| j.as_ref().unwrap().to_owned())
            .collect();

        cache.update(apartments);

        // println!("cache size is {} before the loop", cache.apartments.len());
        for entry in cache.iter() {
            let mut record: ApartmentRecrod = entry.1.apartment.to_owned().into();
            let mut skip = false;

            let a = &entry.1.apartment;
            if entry.1.lifecycle == ApartmentLifeCycle::Sent {
                skip = true;
            } else if !a.elevator
                && !a.description.clone().unwrap_or_default().elevator
                && *a.floor.as_ref().unwrap_or_else(|| &(-1 as i64)) > 2
            {
                log::trace!("Skip due to elevator conditions");
                skip = true;
            } else if ApartmentRecrod::select_one_exp_by(&record.id).is_ok() {
                // Already has record for this id
                entry.1.lifecycle = ApartmentLifeCycle::Sent;
                skip = true;
            } else {
                log::warn!("Record id:{} is not there", record.id.value);
            }
            log::trace!(
                "Skip:{}, Apartmend id:{}, lifecycle:{:?}",
                skip,
                entry.1.apartment.id,
                entry.1.lifecycle
            );
            if !skip {
                let brief = format!(
                    "цена:{}, комн:{}, пл.:{} м2, дист:{} м, этаж:{}, лифт:{}, п.м.:{}, \nоп(л:{}, п:{}, б:{})",
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
                );

                record.brief.value = brief.clone();
                if let Err(e) = record.insert() {
                    log::error!("Fail to save record to the db: {}", e);
                }

                let msg = format!("дата:{} \n{} \nссылка:{}", a.datetime, brief, a.url);
                log::info!("Sending new apartment: id({}), url({})", a.id, a.url);
                // log::info!("Send message: {}", msg);
                entry.1.lifecycle = if chat_id_opt.is_some() {
                    if bot
                        .send_message(chat_id_opt.clone().unwrap(), msg)
                        .await
                        .is_ok()
                    {
                        ApartmentLifeCycle::Sent
                    } else {
                        entry.1.lifecycle
                    }
                } else {
                    ApartmentLifeCycle::Sent
                }
            }

            entry.1.expired = true;
            // println!("send");
        }
        // println!("cache size is {} after the loop", cache.apartments.len());
        // println!("sleep");
        tokio::time::sleep(tokio::time::Duration::from_secs(60 * 10)).await;
    }
    // return Ok(());
}

async fn handle_page(apr: ApartmentPageRequest, client: Arc<reqwest::Client>) -> Option<Apartment> {
    let page_res = apr.request(client).await.unwrap().parse();
    match page_res {
        Ok(page) => Some(page),
        Err(e) => {
            log::error!("Error during parse of a page '{}': {}", apr.href, e);
            None
        }
    }
}
