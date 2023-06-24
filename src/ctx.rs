use object::Object;

pub struct Module {
    base: u64,
    entry_point: u64,
    tls_callback: Vec<u64>,
}

impl Module {
    pub fn new(data: Vec<u8>) -> Self {
        let object = object::read::File::parse(data.as_slice()).unwrap();
        Self {
            base: object.relative_address_base(),
            entry_point: object.entry(),
            tls_callback: vec![],
        }
    }
}
