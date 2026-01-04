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
            let req = line.split(|x| *x == b' ').collect::<Vec<&[u8]>>();
            if req.len() < 2 {
                None
            } else {
                Some((req[0], req[1]))
            }
        }) {
            let mut headers = HashMap::new();
            lines.skip_while(|x| x.is_empty()).take_while(|x| !x.is_empty()).for_each(|line| {
                let h = line.splitn(2, |x| *x == b':').map(|x| x.trim_ascii()).collect::<Vec<&[u8]>>();
                if h.len() == 2 {
                    headers.insert(HeaderKey(h[0]), h[1]);
                } else {
                    eprintln!("Failed parsing header: {:?}", h);
                }
            });

            return Some(Request { method, path, headers })
        }

        None
    }
}