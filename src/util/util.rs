use std::any::type_name;

pub fn print_type_of<T>(_: &T) { println!("{}", type_name::<T>()) }
