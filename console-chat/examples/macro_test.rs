#![allow(dead_code)]
use from_hashmap_macro::FromHashmap;
use std::collections::HashMap;

#[derive(Default, Clone, Debug)]
struct TestType();

#[derive(FromHashmap, Debug)]
#[hashmap(type = "TestType")]
struct Test {
    test: TestType,
    test2: TestType,
    test3: TestType,
    inserted: bool,
}

fn main() {
    let mut map = HashMap::new();
    println!("map (before): {:#?}", map);
    let x = Test::from(&mut map);
    println!("Test object: {:#?}", x);
    println!("map (after): {:#?}", map);
}
