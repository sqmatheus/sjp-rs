#![allow(dead_code, unused)]
#[macro_use]
use std::{collections::HashMap, fs};

use thiserror::Error;

#[derive(Debug, PartialEq)]
enum JsonValue {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<JsonValue>),
    Object(HashMap<String, JsonValue>),
}

struct JsonParser {
    chars: Vec<char>,
    cursor: usize,
}

#[derive(Error, Debug, PartialEq)]
pub enum JsonParserError {
    #[error("expected end of a value")]
    NoEnd,
    #[error("expected `{0}` got `{1}`")]
    InvalidChar(char, char),
    #[error("invalid number `{0}`")]
    InvalidNumber(String),
    #[error("end of file")]
    Eof,
    #[error("unknown json parser error")]
    Unknown,
}

type JsonResult = Result<JsonValue, JsonParserError>;

impl JsonParser {
    fn new(chars: Vec<char>) -> Self {
        JsonParser { chars, cursor: 0 }
    }

    fn chop(&mut self) {
        while let Some(char) = self.chars.get(self.cursor) {
            if char.is_whitespace() {
                self.cursor += 1
            } else {
                break;
            }
        }
    }

    fn read(&mut self) -> Result<char, JsonParserError> {
        if let Some(ch) = self.chars.get(self.cursor) {
            return Ok(*ch);
        }
        Err(JsonParserError::Eof)
    }

    fn consume(&mut self) -> Result<char, JsonParserError> {
        let res = self.read()?;
        self.cursor += 1;
        Ok(res)
    }

    fn consume_check(&mut self, expected: char) -> Result<(), JsonParserError> {
        let got = self.consume()?;
        if got != expected {
            return Err(JsonParserError::InvalidChar(expected, got));
        }
        Ok(())
    }

    fn parse_string(&mut self) -> Result<String, JsonParserError> {
        self.consume_check('"')?;
        let mut end = false;
        let mut text = String::new();
        while self.cursor < self.chars.len() {
            let char = self.consume()?;
            if char == '"' {
                end = true;
                break;
            }
            text.push(char);
        }
        if end {
            Ok(text)
        } else {
            Err(JsonParserError::NoEnd)
        }
    }

    fn parse_next(&mut self) -> JsonResult {
        self.chop();
        let char = self.read()?;
        match char {
            '{' => self.parse_object(),
            '"' => Ok(JsonValue::String(self.parse_string()?)),
            '[' => self.parse_array(),
            _ => {
                let mut text = String::new();
                if char.is_numeric() {
                    let mut found_point = false;
                    while self.cursor < self.chars.len() {
                        let char = self.read()?;
                        if char == '.' {
                            if !found_point {
                                found_point = true
                            } else {
                                text.push(char);
                                return Err(JsonParserError::InvalidNumber(text));
                            }
                        } else if !char.is_numeric() {
                            break;
                        }
                        self.cursor += 1;
                        text.push(char);
                    }
                    let number = match text.parse::<f64>() {
                        Ok(n) => n,
                        Err(_) => return Err(JsonParserError::InvalidNumber(text)),
                    };
                    return Ok(JsonValue::Number(number));
                }

                if char.is_alphabetic() {
                    while self.cursor < self.chars.len() {
                        let char = self.read()?;
                        if !char.is_alphabetic() {
                            break;
                        }
                        text.push(char);
                        self.cursor += 1
                    }
                    return match text.as_str() {
                        "null" => Ok(JsonValue::Null),
                        "true" => Ok(JsonValue::Bool(true)),
                        "false" => Ok(JsonValue::Bool(false)),
                        _ => Err(JsonParserError::Unknown),
                    };
                }
                Err(JsonParserError::Unknown)
            }
        }
    }

    fn parse_array(&mut self) -> JsonResult {
        self.consume_check('[')?;
        self.chop();

        let mut expect_next = false;
        let mut end = false;
        let mut result = Vec::<JsonValue>::new();

        while self.cursor < self.chars.len() {
            if self.read()? == ']' {
                if expect_next {
                    return Err(JsonParserError::Unknown);
                }
                end = true;
                self.cursor += 1;
                break;
            }

            result.push(self.parse_next()?);

            self.chop();
            let next = self.consume()?;
            if next != ',' {
                if next == ']' {
                    end = true;
                }
                break;
            }
            expect_next = true;
            self.chop();
        }
        if end {
            self.chop();
            Ok(JsonValue::Array(result))
        } else {
            Err(JsonParserError::NoEnd)
        }
    }

    fn parse_object(&mut self) -> JsonResult {
        self.consume_check('{')?;
        self.chop();

        let mut expect_next = false;
        let mut end = false;
        let mut result = HashMap::<String, JsonValue>::new();

        while self.cursor < self.chars.len() {
            if self.read()? == '}' {
                if expect_next {
                    return Err(JsonParserError::Unknown);
                }
                end = true;
                self.cursor += 1;
                break;
            }

            let key = self.parse_string()?;

            self.chop();
            self.consume_check(':')?;

            self.chop();
            result.insert(key, self.parse_next()?);

            self.chop();
            let next = self.consume()?;
            if next != ',' {
                if next == '}' {
                    end = true;
                }
                break;
            }
            expect_next = true;
            self.chop();
        }
        if end {
            Ok(JsonValue::Object(result))
        } else {
            Err(JsonParserError::NoEnd)
        }
    }

    fn parse(&mut self) -> JsonResult {
        self.chop();
        self.parse_object()
    }
}

fn parse_file(file_path: &str) -> JsonResult {
    let content: Vec<char> = match fs::read_to_string(file_path) {
        Ok(v) => v.chars().collect(),
        Err(_) => return Err(JsonParserError::Unknown),
    };
    let mut parser = JsonParser::new(content);
    parser.parse()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all() {
        let result = parse_file("./test.json");
        let mut hash = HashMap::new();

        hash.insert("hello".to_string(), JsonValue::String("world".to_string()));

        hash.insert("number".to_string(), JsonValue::Number(100.0));

        hash.insert("null".to_string(), JsonValue::Null);

        hash.insert("true".to_string(), JsonValue::Bool(true));

        hash.insert("false".to_string(), JsonValue::Bool(false));

        let vec = vec![JsonValue::Null];
        hash.insert("array".to_string(), JsonValue::Array(vec));

        if let Err(e) = &result {
            println!("{}", e);
        }

        println!("{:?}", result);

        assert!(result == Ok(JsonValue::Object(hash)))
    }

    // TODO: more tests
}
