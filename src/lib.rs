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

    fn consume_check(&mut self, expected: char) -> Result<char, JsonParserError> {
        let got = self.consume()?;
        if got != expected {
            return Err(JsonParserError::InvalidChar(expected, got));
        }
        Ok(got)
    }

    fn parse_string(&mut self) -> Result<String, JsonParserError> {
        self.consume_check('"')?;
        let mut end = false;
        let mut text = String::new();
        while let Ok(ch) = self.consume() {
            if ch == '"' {
                end = true;
                break;
            }
            text.push(ch);
        }
        if end {
            Ok(text)
        } else {
            Err(JsonParserError::NoEnd)
        }
    }

    fn parse_number(&mut self) -> Result<f64, JsonParserError> {
        let mut buffer = String::new();
        let mut found_point = false;
        while let Ok(ch) = self.read() {
            if !ch.is_numeric() && ch != '.' {
                break;
            }
            self.cursor += 1;
            buffer.push(ch);
        }
        Ok(buffer
            .parse::<f64>()
            .map_err(|_| JsonParserError::InvalidNumber(buffer))?)
    }

    fn parse_identifier(&mut self) -> JsonResult {
        let mut buffer = String::new();
        while let Ok(ch) = self.read() {
            if !ch.is_alphabetic() {
                break;
            }
            buffer.push(ch);
            self.cursor += 1
        }
        return match buffer.as_str() {
            "null" => Ok(JsonValue::Null),
            "true" => Ok(JsonValue::Bool(true)),
            "false" => Ok(JsonValue::Bool(false)),
            _ => Err(JsonParserError::Unknown),
        };
    }

    fn parse_next(&mut self) -> JsonResult {
        self.chop();
        let char = self.read()?;
        match char {
            '{' => self.parse_object(),
            '"' => Ok(JsonValue::String(self.parse_string()?)),
            '[' => self.parse_array(),
            _ => {
                if char.is_numeric() {
                    return Ok(JsonValue::Number(self.parse_number()?));
                }
                if char.is_alphabetic() {
                    return Ok(self.parse_identifier()?);
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
        let mut result = Vec::new();

        while let Ok(ch) = self.read() {
            if ch == ']' {
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

        while let Ok(ch) = self.read() {
            if ch == '}' {
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
    let mut parser = JsonParser::new(
        fs::read_to_string(file_path)
            .map_err(|_| JsonParserError::Unknown)?
            .chars()
            .collect(),
    );
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
