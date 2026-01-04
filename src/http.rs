use std::collections::HashMap;
use std::hash::{Hash, Hasher};

#[derive(Debug)]
pub struct HeaderKey<'a>(pub &'a [u8]);

impl<'a> PartialEq for HeaderKey<'a> {
    fn eq(&self, other: &Self) -> bool {
        self.0.eq_ignore_ascii_case(other.0)
    }
}

impl<'a> Eq for HeaderKey<'a> {}

impl<'a> Hash for HeaderKey<'a> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        for &b in self.0 {
            state.write_u8(b.to_ascii_lowercase());
        }
    }
}

#[derive(Debug)]
pub struct Request<'a> {
    pub method: &'a [u8],
    pub path: &'a [u8],
    pub headers: HashMap::<HeaderKey<'a>, &'a [u8]>,
}

impl<'a> Request<'a> {
    pub fn new(buf: &'a [u8]) -> Option<Request<'a>> {
        let mut lines = buf.split(|x| *x == b'\n').map(|x| x.trim_ascii_end());

        if let Some((method, path)) = lines.next().and_then(|line| {
            let mut req = line.split(|x| *x == b' ').into_iter();
            let method = req.next();
            let path = req.next();
            method.and_then(|m| path.map(|p| (m, p)))
        }) {
            let mut headers = HashMap::new();
            lines.skip_while(|x| x.is_empty()).take_while(|x| !x.is_empty()).for_each(|line| {
                let mut h = line.splitn(2, |x| *x == b':').map(|x| x.trim_ascii()).into_iter();
                let header = h.next();
                let value = h.next();
                match header.and_then(|h| value.map(|v| (h, v))) {
                    Some((h, v)) => {
                        headers.insert(HeaderKey(h), v);
                    },
                    None => {
                        eprintln!("Failed parsing header: {:?}", line)
                    },
                }
            });

            return Some(Request { method, path, headers })
        }

        None
    }
}