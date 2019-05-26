#[macro_use]
extern crate lazy_static;
extern crate curl;
extern crate curl_sys;
extern crate percent_encoding;
extern crate scraper;

use curl::easy::{Easy2, Handler, WriteError};
use scraper::ElementRef;
use scraper::{Html, Selector};
use std::borrow::Cow;
use std::convert::TryFrom;
use std::error::Error;
use std::sync::Mutex;
use std::time::Duration;

const GOOGLE_URL: &str = "https://www.google.com/search?source=hp";

lazy_static! {
    static ref LINK_SELECT: Selector =
        Selector::parse("div.mw div.col .bkWMgd div.srg div.g div div.rc div.r a cite.iUh30")
            .unwrap();
    static ref TITLE_SELECT: Selector = Selector::parse(".LC20lb").unwrap();
    static ref DESCRIPTION_SELECT: Selector = Selector::parse("div.s").unwrap();
}

#[derive(Debug, Default)]
pub struct GResult {
    title: String,
    link: String,
    description: String,
}

impl TryFrom<ElementRef<'_>> for GResult {
    type Error = &'static str;
    fn try_from(value: ElementRef) -> Result<Self, Self::Error> {
        let srch_result = value
            .ancestors()
            .nth(4)
            .and_then(ElementRef::wrap)
            .ok_or("Couldnt get search node")?;
        Ok(GResult {
            title: srch_result
                .select(&TITLE_SELECT)
                .next()
                .ok_or("Cant get title")?
                .text()
                .collect::<String>(),
            link: value.text().collect::<String>(),
            description: srch_result
                .select(&DESCRIPTION_SELECT)
                .next()
                .ok_or("Cant get description")?
                .text()
                .collect::<String>(),
        })
    }
}

#[derive(Debug)]
struct Writeback(Vec<u8>);
impl Handler for Writeback {
    fn write(&mut self, data: &[u8]) -> Result<usize, WriteError> {
        self.0.extend_from_slice(data);
        Ok(data.len())
    }
}

lazy_static! {
    static ref CLIENT: Mutex<Easy2<Writeback>> = {
        let x = Mutex::new(Easy2::new(Writeback(Vec::new())));
        let mut held = x.lock().unwrap();
        held.get(true).unwrap();
        held.dns_cache_timeout(Duration::from_secs(60*60*3)).unwrap();
        held.accept_encoding("").unwrap();
        held.follow_location(true).unwrap();
        held.useragent("Mozilla/5.0 (Windows NT 10.0; WOW64; Trident/7.0; rv:11.0) like Gecko").unwrap(); // Aw yea
        // held.cookie_file("./cookies.txt").unwrap();
        // held.cookie_jar("./cookies.txt").unwrap();
        held.timeout(Duration::from_secs(20)).unwrap();
        held.connect_timeout(Duration::from_secs(20)).unwrap();
        held.http_version(curl::easy::HttpVersion::V2TLS).unwrap();
        held.ssl_version(curl::easy::SslVersion::Tlsv13).unwrap();
        unsafe { assert_eq!(curl_sys::curl_easy_setopt(held.raw(), curl_sys::CURLUSESSL_ALL), curl_sys::CURLE_OK);};
        drop(held);
        x
    };
}

pub fn google(query: &str) -> Result<Vec<GResult>, Box<dyn Error>> {
    let mut held = CLIENT.lock().unwrap();
    held.url(
        &[
            GOOGLE_URL,
            "&q=",
            &percent_encoding::utf8_percent_encode(query, percent_encoding::QUERY_ENCODE_SET)
                .collect::<Cow<str>>(),
        ]
        .concat(),
    )
    .unwrap();
    held.perform().unwrap();
    let result = Html::parse_document(&String::from_utf8_lossy(&held.get_ref().0))
        .select(&LINK_SELECT)
        .take(3)
        .map(GResult::try_from)
        .collect::<Result<Vec<GResult>, &'static str>>()?;
    held.get_mut().0 = Vec::new(); // clearning dat state, also this sucks
    Ok(result)
}
