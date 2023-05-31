use crate::db::utils::DatabaseSource;

pub struct Config {}
impl Config {
    pub fn database_location() -> DatabaseSource {
        DatabaseSource::File("rentbot_sslv.db".into())
    }
    pub fn price_low() -> u32 {
        300
    }
    pub fn price_high() -> u32 {
        1200
    }
    pub fn area_low() -> u32 {
        70
    }
}
