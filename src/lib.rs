use js_sys::{Error, Function, JsString, TypeError};
use pulldown_cmark::{CowStr, Event, Options, Parser, Tag};
use pulldown_cmark_to_cmark::fmt::cmark;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

#[allow(dead_code)]
fn example(markdown_input: &str) -> Vec<String> {
    // Set up options and parser. Strikethroughs are not part of the CommonMark standard
    // and we therefore must enable it explicitly.
    let mut opts = Options::empty();
    opts.insert(Options::ENABLE_TABLES);
    opts.insert(Options::ENABLE_FOOTNOTES);
    opts.insert(Options::ENABLE_STRIKETHROUGH);
    opts.insert(Options::ENABLE_TASKLISTS);
    let parser = Parser::new_ext(markdown_input, opts);
    let mut re: Vec<String> = Vec::new();
    for event in parser.clone() {
        match event {
            Event::Start(Tag::Image(_, url, _)) => {
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
    initial_capacity: usize,
}
#[wasm_bindgen]
impl MarkdownImgUrlEditor {
    #[wasm_bindgen(constructor)]
    pub fn new(text: String, converter: &Function) -> Result<MarkdownImgUrlEditor, JsValue> {
        let markdown_text = text.clone();
        // Set up options and parser. Strikethroughs are not part of the CommonMark standard
        // and we therefore must enable it explicitly.
        let mut opts = Options::empty();
        opts.insert(Options::ENABLE_TABLES);
        opts.insert(Options::ENABLE_FOOTNOTES);
        opts.insert(Options::ENABLE_STRIKETHROUGH);
        opts.insert(Options::ENABLE_TASKLISTS);
        let parser = Parser::new_ext(&markdown_text, opts);
        let mut string_generators: Vec<Function> = Vec::new();
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
                Event::End(Tag::Image(_, u, _)) => {
                    in_image = false;
                    let mut a: Option<String> = None;
                    std::mem::swap(&mut alt, &mut a);
                    let alt = JsValue::from(a.unwrap());
                    let url = JsValue::from(u.into_string());
                    let generator = converter.call2(&JsValue::NULL, &alt, &url);
                    match generator {
                        Ok(maybe_g) => match maybe_g.dyn_into::<Function>() {
                            Ok(g) => {
                                string_generators.push(g);
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
        Ok(MarkdownImgUrlEditor {
            markdown_text,
            string_generators,
            initial_capacity: text.len() + 128,
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
        let parser = Parser::new_ext(&self.markdown_text, Options::empty());
        let mut ite = urls.iter();
        let modified = parser.map(|e| match e {
            Event::End(tag) => Event::End(match tag {
                Tag::Image(link_type, _, title) => {
                    Tag::Image(link_type, CowStr::from(ite.next().unwrap().clone()), title)
                }
                _ => tag,
            }),
            _ => e,
        });
        let mut buf = String::with_capacity(self.initial_capacity);
        cmark(modified, &mut buf, None).map_err(|_| Error::new("cmark failed."))?;
        Ok(buf)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn example() {
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
