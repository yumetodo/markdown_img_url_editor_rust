use js_sys::{Function, JsString, RangeError, TypeError};
use pulldown_cmark::{Event, Options, Parser, Tag};
use std::ops::Range;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
extern crate console_error_panic_hook;

#[allow(dead_code)]
fn example(markdown_input: &str) -> Vec<String> {
    // Set up options and parser. Strikethroughs are not part of the CommonMark standard
    // and we therefore must enable it explicitly.
    let mut opts = Options::empty();
    opts.insert(Options::ENABLE_TABLES);
    opts.insert(Options::ENABLE_FOOTNOTES);
    opts.insert(Options::ENABLE_STRIKETHROUGH);
    opts.insert(Options::ENABLE_TASKLISTS);
    let parser = Parser::new_ext(markdown_input, opts).into_offset_iter();
    let mut re: Vec<String> = Vec::new();
    for (event, range) in parser {
        match event {
            Event::End(Tag::Image(_, url, _)) => {
                let all = &markdown_input[range.start..range.end];
                let i = all.rfind(&url.clone().into_string()).unwrap();
                let url_part = &all[i..(i + url.len())];
                println!(
                    "start: {}, end: {}, s={}, part={}",
                    range.start, range.end, all, url_part
                );
                re.push(url.into_string());
            }
            _ => (),
        }
    }
    re
}
#[allow(dead_code)]
fn example2(markdown_input: &str) -> Vec<String> {
    // Set up options and parser. Strikethroughs are not part of the CommonMark standard
    // and we therefore must enable it explicitly.
    let mut opts = Options::empty();
    opts.insert(Options::ENABLE_TABLES);
    opts.insert(Options::ENABLE_FOOTNOTES);
    opts.insert(Options::ENABLE_STRIKETHROUGH);
    opts.insert(Options::ENABLE_TASKLISTS);
    let parser = Parser::new_ext(markdown_input, opts);
    let mut re: Vec<String> = Vec::new();
    let mut in_image = false;
    let mut alt: Option<String> = None;
    for event in parser {
        match event {
            Event::Start(Tag::Image(_, _, _)) => {
                in_image = true;
            }
            Event::Text(t) => {
                if alt.is_some() {
                    let mut tmp = alt.unwrap();
                    tmp.push(' ');
                    alt = Some(tmp + &t.into_string());
                } else if in_image {
                    alt = Some(t.into_string());
                }
            }
            Event::End(Tag::Image(link_type, u, _)) => {
                in_image = false;
                let mut a: Option<String> = None;
                std::mem::swap(&mut alt, &mut a);
                println!("{:?}, {:?}, {:?}", link_type, u, a);
                re.push(a.unwrap());
            }
            _ => (),
        }
    }
    re
}
#[wasm_bindgen]
pub struct MarkdownImgUrlEditor {
    markdown_text: String,
    string_generators: Vec<Function>,
    url_ranges: Vec<Range<usize>>,
    without_replace_part_len: usize,
}

fn calc_url_range<'a>(markdown_text:&'a str, url: &'a str, range: Range<usize>) -> Range<usize> {
    let all = &markdown_text[range.start..range.end];
    let i = all.rfind(url).unwrap() + range.start;
    i..(i + url.len())
}
#[wasm_bindgen]
impl MarkdownImgUrlEditor {
    #[wasm_bindgen(constructor)]
    pub fn new(text: String, converter: &Function) -> Result<MarkdownImgUrlEditor, JsValue> {
        console_error_panic_hook::set_once();
        let markdown_text = text.clone();
        // Set up options and parser. Strikethroughs are not part of the CommonMark standard
        // and we therefore must enable it explicitly.
        let mut opts = Options::empty();
        opts.insert(Options::ENABLE_TABLES);
        opts.insert(Options::ENABLE_FOOTNOTES);
        opts.insert(Options::ENABLE_STRIKETHROUGH);
        opts.insert(Options::ENABLE_TASKLISTS);
        let parser = Parser::new_ext(&markdown_text, opts).into_offset_iter();
        let mut string_generators: Vec<Function> = Vec::new();
        let mut url_ranges: Vec<Range<usize>> = Vec::new();
        let mut in_image = false;
        let mut alt: Option<String> = None;
        let mut prev_url_end: usize = 0;
        let mut without_replace_part_len: usize = 0;
        for (event, range) in parser {
            match event {
                Event::Start(Tag::Image(_, _, _)) => {
                    in_image = true;
                }
                Event::Text(t) => {
                    if alt.is_some() {
                        let mut tmp = alt.unwrap();
                        tmp.push(' ');
                        alt = Some(tmp + &t.into_string());
                    } else if in_image {
                        alt = Some(t.into_string());
                    }
                }
                Event::End(Tag::Image(_, u, _)) => {
                    in_image = false;
                    let mut a: Option<String> = None;
                    std::mem::swap(&mut alt, &mut a);
                    let url = u.into_string();
                    let url_range = calc_url_range(&markdown_text, &url, range);
                    let alt = JsValue::from(a.unwrap());
                    let generator = converter.call2(&JsValue::NULL, &alt, &JsValue::from(url));
                    match generator {
                        Ok(maybe_g) => match maybe_g.dyn_into::<Function>() {
                            Ok(g) => {
                                string_generators.push(g);
                                if url_range.start < prev_url_end {
                                    return Err(JsValue::from(RangeError::new(&format!(
                                        "url_range.start: {}, prev_url_end: {}",
                                        url_range.start, prev_url_end
                                    ))));
                                }
                                without_replace_part_len += url_range.start - prev_url_end;
                                prev_url_end = url_range.end;
                                url_ranges.push(url_range);
                            }
                            Err(_) => {
                                return Err(JsValue::from(TypeError::new(
                                    "`converter` (2nd argument): expected Function",
                                )));
                            }
                        },
                        Err(m) => {
                            return Err(m);
                        }
                    }
                }
                _ => (),
            }
        }
        without_replace_part_len += markdown_text.len() - prev_url_end;
        Ok(MarkdownImgUrlEditor {
            markdown_text,
            string_generators,
            url_ranges,
            without_replace_part_len,
        })
    }
    pub fn replace(&mut self) -> Result<String, JsValue> {
        if self.string_generators.is_empty() {
            return Ok(self.markdown_text.clone());
        }
        let urls = self
            .string_generators
            .clone()
            .into_iter()
            .map(|g| {
                g.call0(&JsValue::NULL).and_then(|s| {
                    s.dyn_into::<JsString>()
                        .map(|s| String::from(s))
                        .map_err(|_| {
                            JsValue::from(TypeError::new(
                                "before_collect_callback (3rd argument: expected Function",
                            ))
                        })
                })
            })
            .collect::<Result<Vec<String>, JsValue>>()?;
        let mut buf = String::with_capacity(
            self.without_replace_part_len + urls.iter().map(|e| e.len()).sum::<usize>(),
        );
        let mut prev_url_end = 0;
        for (r, url) in self.url_ranges.iter().zip(urls.iter()) {
            buf += &self.markdown_text[prev_url_end..r.start];
            buf += url;
            prev_url_end = r.end;
        }
        buf += &self.markdown_text[prev_url_end..];
        Ok(buf)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn example() {
        let markdown_input = r"# arikitari

![🍣🍺1](1.png)

![cpp](D3T3cG6U0AAd0Zn.jpg)

![atgtheiwa1](D5iuwp0W4AEm1bi.jpg)

![atgtheiwa2](D5jwkn7XsAABJvV.jpg)

![IR](IR.jpg)
```markdown
![2][2.png]
```
";
        let re = super::example(markdown_input);
        let test = re.iter().eq(&[
            "1.png",
            "D3T3cG6U0AAd0Zn.jpg",
            "D5iuwp0W4AEm1bi.jpg",
            "D5jwkn7XsAABJvV.jpg",
            "IR.jpg",
        ]);
        assert!(test);
    }
    #[test]
    fn example2() {
        let markdown_input = r"# arikitari

![1](1.png)

![cpp](D3T3cG6U0AAd0Zn.jpg)

![atgtheiwa1](D5iuwp0W4AEm1bi.jpg)

![atgtheiwa2](D5jwkn7XsAABJvV.jpg)

![IR](IR.jpg)
```markdown
![2][2.png]
```
";
        let re = super::example2(markdown_input);
        print!("{:?}", re);
        let test = re
            .iter()
            .eq(&["1", "cpp", "atgtheiwa1", "atgtheiwa2", "IR"]);
        assert!(test);
    }
}
