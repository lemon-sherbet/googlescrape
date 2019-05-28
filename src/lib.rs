use curl::easy::{Easy2, Handler, WriteError};
use lazy_static::lazy_static;
use scraper::ElementRef;
use scraper::{Html, Selector};
use std::borrow::Cow;
use std::convert::TryFrom;
use std::error::Error;
#[cfg(feature = "ffi")]
use std::ffi::CString;
use std::mem;
use std::sync::Mutex;
use std::time::Duration;

#[cfg(feature = "ffi")]
mod ffi;
#[cfg(feature = "ffi")]
use ffi::*;

const GOOGLE_URL: &str = "https://www.google.com/search?source=hp";

lazy_static! {
    static ref LINK_SELECT: Selector =
        Selector::parse("div.mw div.col .bkWMgd div.srg div.g div div.rc div.r a cite.iUh30")
            .unwrap();
    static ref TITLE_SELECT: Selector = Selector::parse(".LC20lb").unwrap();
    static ref DESCRIPTION_SELECT: Selector = Selector::parse("div.s").unwrap();
}

#[cfg(not(feature = "ffi"))]
#[derive(Debug)]
pub struct GResult {
    title: String,
    link: String,
    description: String,
}

impl TryFrom<ElementRef<'_>> for GResult {
    type Error = Box<dyn Error + Send + Sync>;
    fn try_from(value: ElementRef) -> Result<Self, Self::Error> {
        let srch_result = value
            .ancestors()
            .nth(4)
            .and_then(ElementRef::wrap)
            .ok_or("Couldnt get search node")?;
        let title = srch_result
            .select(&TITLE_SELECT)
            .next()
            .ok_or("Cant get title")?
            .text()
            .collect::<String>();
        let link = value.text().collect::<String>();
        let description = srch_result
            .select(&DESCRIPTION_SELECT)
            .next()
            .ok_or("Cant get description")?
            .text()
            .collect::<String>();
        Ok(
            #[cfg(not(feature = "ffi"))]
            GResult {
                title,
                link,
                description,
            },
            #[cfg(feature = "ffi")]
            GResult {
                title: CString::new(title)?.into_raw(),
                link: CString::new(link)?.into_raw(),
                description: CString::new(description)?.into_raw(),
            },
        )
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
    static ref CLIENT: Result<Mutex<Easy2<Writeback>>, String> = {
        let lock = Mutex::new(Easy2::new(Writeback(Vec::new())));
        if let Err(x) = (|| -> Result<(), Box<dyn Error + Send + Sync>> {
            let mut held = lock.lock().map_err(|x| x.to_string())?;
            held.get(true)?;
            held.dns_cache_timeout(Duration::from_secs(60 * 60 * 3))?;
            held.accept_encoding("")?;
            held.follow_location(true)?;
            held.useragent(
                "Mozilla/5.0 (Windows NT 10.0; WOW64; Trident/7.0; rv:11.0) like Gecko",
            )?;
            held.timeout(Duration::from_secs(20))?;
            held.connect_timeout(Duration::from_secs(20))?;
            // held.http_version(curl::easy::HttpVersion::V2TLS)?;
            // held.ssl_version(curl::easy::SslVersion::Tlsv13)?;
            // unsafe {
            //     match curl_sys::curl_easy_setopt(held.raw(), curl_sys::CURLUSESSL_ALL) {
            //         curl_sys::CURLE_OK => Ok(()),
            //         x => Err(curl::Error::new(x)),
            //     }
            // }?;
            // For some reason this doesnt work and I get unsupported protocol sometimes
            mem::drop(held); // Locks are scary, so are macros, lets be careful :)
            Ok(())
        })() {
            return Err(x.to_string());
        };
        Ok(lock)
    };
}

#[cfg(not(feature = "ffi"))]
pub fn google(query: &str) -> Result<Vec<GResult>, Box<dyn Error + Send + Sync>> {
    _google(query)
}

#[inline(always)]
fn _google(query: &str) -> Result<Vec<GResult>, Box<dyn Error + Send + Sync>> {
    let mut held = match &*CLIENT {
        Ok(x) => x,
        Err(x) => return Err(Box::from(x.as_str())),
    }
    .lock()
    .unwrap();
    held.url(
        &[
            GOOGLE_URL,
            "&q=",
            &percent_encoding::utf8_percent_encode(query, percent_encoding::QUERY_ENCODE_SET)
                .collect::<Cow<str>>(),
        ]
        .concat(),
    )?;
    held.perform()?;
    let result = Html::parse_document(&String::from_utf8_lossy(&held.get_ref().0))
        .select(&LINK_SELECT)
        .take(3)
        .map(GResult::try_from)
        .collect::<Result<Vec<GResult>, Box<dyn Error + Send + Sync>>>()?;
    assert_eq!(result.len(), 3);
    assert_eq!(result.capacity(), 4);
    held.get_mut().0 = Vec::new(); // clearning dat state, also this sucks
    Ok(result)
}
