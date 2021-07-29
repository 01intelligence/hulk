#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use serde::{Deserialize, Serialize};

    use super::super::*;

    fn fmt_xml<T: AsRef<str>>(s: T) -> String {
        format!("\n**** XML ****\n{}\n**** XML ****\n", s.as_ref())
    }

    #[test]
    fn test_xml() {
        #[derive(Serialize, Deserialize)]
        struct UnitStruct;
        #[derive(Serialize, Deserialize)]
        struct NewTypeStruct(&'static str);
        #[derive(Serialize, Deserialize)]
        struct TupleStruct(&'static str, i32);
        #[derive(Serialize, Deserialize)]
        struct SimpleStruct {
            i32: i32,
            str: &'static str,
        }
        #[derive(Serialize, Deserialize)]
        enum Enum {
            W { a: i32, b: i32 },
            X(i32, i32),
            Y(i32),
            Z,
        }
        #[derive(Serialize, Deserialize)]
        struct Struct {
            #[serde(rename = "wrap $name")]
            wrap: &'static str,
            string: String,
            str: &'static str,
            usize: usize,
            i64: i64,
            bool: bool,
            f64: f64,
            none: Option<isize>,
            some: Option<u64>,
            vec: Vec<u64>,
            map: HashMap<i32, &'static str>,
            unit: UnitStruct,
            new_type_struct: NewTypeStruct,
            tuple_struct: TupleStruct,
            simple_struct: SimpleStruct,
        }

        let e = UnitStruct {};
        let s = xml::to_string(&e).unwrap();
        println!("UnitStruct: {}", fmt_xml(s));

        let e = NewTypeStruct("hello");
        let s = xml::to_string(&e).unwrap();
        println!("NewTypeStruct: {}", fmt_xml(s));

        let e = Enum::W { a: 0, b: 0 };
        let s = xml::to_string(&e).unwrap();
        println!("Enum: {}", fmt_xml(s));
        let e = Enum::X(0, 0);
        let s = xml::to_string(&e).unwrap();
        println!("Enum: {}", fmt_xml(s));
        let e = Enum::Y(0);
        let s = xml::to_string(&e).unwrap();
        println!("Enum: {}", fmt_xml(s));
        let e = Enum::Z;
        let s = xml::to_string(&e).unwrap();
        println!("Enum: {}", fmt_xml(s));

        let e = Struct {
            wrap: "wrap",
            string: "hello".to_owned(),
            str: "world",
            usize: 32,
            i64: -64,
            bool: true,
            f64: 0.64,
            none: None,
            some: Some(64),
            vec: vec![1, 2, 3],
            map: maplit::hashmap! {
                1 => "one",
                2 => "two",
            },
            unit: UnitStruct,
            new_type_struct: NewTypeStruct("hello"),
            tuple_struct: TupleStruct("world", 32),
            simple_struct: SimpleStruct {
                i32: 32,
                str: "str",
            },
        };
        let s = xml::to_string(&e).unwrap();
        println!("{}", fmt_xml(s));
        let s = xml::to_string_with_indent(&e, b' ', 2).unwrap();
        println!("{}", fmt_xml(s));
    }
}
