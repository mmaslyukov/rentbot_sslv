use crate::db::record::ApartmentRecrod;
use derive_builder::Builder;

#[derive(Debug, Clone)]
pub struct Location {
    pub latitude: f64,
    pub longitude: f64,
}

#[derive(Debug, Clone, Default)]
pub struct ApartmentDescription {
    pub park: bool,
    pub elevator: bool,
    pub balkony: bool,
}

impl ApartmentDescription {
    pub fn new(park: bool, elevator: bool, balkony: bool) -> Self {
        Self {
            park,
            balkony,
            elevator,
        }
    }
}

#[derive(Default, Debug, Builder, Clone)]
pub struct Apartment {
    pub url: String,
    pub id: String,
    pub price: String,
    pub datetime: chrono::NaiveDateTime,
    pub city: String,
    pub district: String,
    pub address: String,
    #[builder(default)]
    pub location: Option<Location>,
    #[builder(default)]
    pub distance: Option<i64>,
    pub rooms: u64,
    pub area: f64,
    pub floor: Option<i64>,
    pub elevator: bool,
    pub parking: bool,
    #[builder(default)]
    pub description: Option<ApartmentDescription>,
}

impl From<Apartment> for ApartmentRecrod {
    fn from(value: Apartment) -> Self {
        let mut record = ApartmentRecrod::new();
        record.id.value = value.id;
        record.datetime.value = value.datetime.to_string();
        record.price.value = value.price;
        record.url.value = value.url;
        record
    }
}
