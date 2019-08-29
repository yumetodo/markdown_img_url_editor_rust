extern crate pulldown_cmark;
extern crate pulldown_cmark_to_cmark;
extern crate wasm_bindgen;
extern crate wasm_bindgen_futures;
extern crate futures;
extern crate js_sys;

use std::io::{stdout, Write};
use pulldown_cmark::{ Options, Parser, Event, Tag, CowStr };
use pulldown_cmark_to_cmark::fmt::cmark;
use wasm_bindgen::prelude::*;
use js_sys::{JsString, Function, Promise, Error, TypeError};
use wasm_bindgen::JsCast;


#[allow(dead_code)]
fn example(markdown_input: &str) -> Vec<String> {
    // Set up options and parser. Strikethroughs are not part of the CommonMark standard
    // and we therefore must enable it explicitly.
    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);
    let parser = Parser::new_ext(markdown_input, options);
    let mut re: Vec<String> = Vec::new();
    for event in parser.clone() {
        match event {
            Event::Start(Tag::Image(_, url, _)) => {
                re.push(url.into_string());
            }
            _ => ()
        }
    }
    let modified = parser.map(|e| {
        match e {
            Event::End(tag) => Event::End(match tag {
                Tag::Image(link_type, _, title) => Tag::Image(link_type, CowStr::from("aaa"), title),
                _ => tag,
            }),
            _ => e,
        }
    });
    let mut buf = String::with_capacity(markdown_input.len() + 128);
    cmark(modified, &mut buf, None).unwrap();
    stdout().write_all(buf.as_bytes()).unwrap();
    re
}
fn get_replaced(parser: Parser, string_generators: Vec<Function>, initial_capacity: usize) -> Result<JsString, JsValue> {
    let a = [1, 2, 3];

    // the checked sum of all of the elements of the array
    let sum = a.iter().try_fold(0i8, |acc, &x| acc.checked_add(x));
    let urls = string_generators.into_iter().map(
        |g| g.call0(&JsValue::NULL).and_then(
            |s| s.dyn_into::<JsString>().map_err(
                |_| JsValue::from(TypeError::new("before_collect_callback (3rd argument: expected Function"))
            )
        )
    ).collect::<Result<Vec<JsString>, JsValue>>();
    let modified = parser.map(|e| {
        match e {
            Event::End(tag) => Event::End(match tag {
                Tag::Image(link_type, _, title) => Tag::Image(link_type, CowStr::from("aaa"), title),
                _ => tag,
            }),
            _ => e,
        }
    });
    let mut buf = String::with_capacity(initial_capacity);
    cmark(modified, &mut buf, None).map_err(|_| Error::new("cmark failed."))?;
    Ok(JsString::from(buf))
}
fn get_replaced_wrap(parser: Parser, string_generators: Vec<Function>, initial_capacity: usize) -> Promise {
    match get_replaced(parser, string_generators, initial_capacity) {
        Ok(s) => Promise::resolve(&s),
        Err(e) => Promise::reject(&e),
    }
}
#[wasm_bindgen]
pub fn markdown_img_url_editor(markdown_text: &str, converter: &Function, before_collect_callback: JsValue) -> Promise {
    let parser = Parser::new_ext(markdown_text, Options::empty());
    let mut string_generators: Vec<Function> = Vec::new();
    for event in parser.clone() {
        match event {
            Event::Start(Tag::Image(_, u, t)) => {
                //TODO: thisは本当にnullでいいのか
                let this = JsValue::NULL;
                let alt = JsValue::from(t.into_string());
                let url = JsValue::from(u.into_string());
                let generator = converter.call2(&this, &alt, &url);
                match generator {
                    Ok(maybe_g) => {
                        match maybe_g.dyn_into::<Function>() {
                            Ok(g) => {
                                string_generators.push(g);
                            },
                            Err(_) => {
                                return Promise::reject(&JsValue::from(TypeError::new("`converter` (2nd argument): expected Function")));
                            }
                        }
                    },
                    Err(m) => {
                        return Promise::reject(&m);
                    }
                }
            }
            _ => ()
        }
    }
    if string_generators.is_empty() {
        return Promise::resolve(&JsValue::from(markdown_text));
    }
    if before_collect_callback.is_null() || before_collect_callback.is_undefined() {
        return get_replaced_wrap(parser, string_generators, markdown_text.len() + 128)
    } else {
        match before_collect_callback.dyn_into::<Function>() {
            Ok(callback) => {
                match callback.call0(&JsValue::NULL) {
                    Ok(maybe_promise) => {
                        if let Ok(p) = maybe_promise.dyn_into::<Promise>() {
                            return p.then(&Closure::wrap(Box::new(move |_| get_replaced_wrap(parser, string_generators, markdown_text.len() + 128))));
                        } else {
                            return Promise::reject(&TypeError::new(""));
                        }
                    },
                    Err(m) => {
                        return Promise::reject(&m);
                    }
                }
            },
            Err(_) => {
                return Promise::reject(&TypeError::new("before_collect_callback (3rd argument: expected Function"));
            }
        }
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
        let test = re.iter().eq(&["1.png", "D3T3cG6U0AAd0Zn.jpg", "D5iuwp0W4AEm1bi.jpg", "D5jwkn7XsAABJvV.jpg", "IR.jpg"]) ;
        assert!(test);
    }
}
