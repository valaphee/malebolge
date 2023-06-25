#[derive(Clone)]
pub struct Address {
    module: Option<String>,
    symbol: Option<String>,
    offset: usize,
}

impl Address {
    pub fn to_raw(self) -> u64 {
        let base = self.module.map_or(0, |module_name| 0);
        let symbol = self
            .symbol
            .map_or(0, |symbol| match symbol.to_lowercase().as_str() {
                _ => todo!(),
            });
        base + symbol + self.offset as u64
    }
}

impl From<&str> for Address {
    fn from(value: &str) -> Self {
        let module_and_symbol_and_offset = value.splitn(2, ':').collect::<Vec<_>>();
        let module;
        let symbol_and_offset;
        if module_and_symbol_and_offset.len() == 1 {
            module = None;
            symbol_and_offset = module_and_symbol_and_offset[0]
        } else {
            module = if module_and_symbol_and_offset[0].is_empty() {
                None
            } else {
                Some(module_and_symbol_and_offset[0].to_owned())
            };
            symbol_and_offset = module_and_symbol_and_offset[1]
        }
        let mut symbol_and_offset = symbol_and_offset.splitn(2, '+');
        let symbol = symbol_and_offset.next().unwrap();
        let symbol = if symbol.is_empty() {
            None
        } else {
            Some(symbol.to_owned())
        };
        let offset = symbol_and_offset
            .next()
            .map(|offset| offset.parse::<usize>().unwrap())
            .unwrap_or(0);
        Self {
            module,
            symbol,
            offset,
        }
    }
}
