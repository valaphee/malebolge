use std::io;

#[cfg(windows)]
use winres::WindowsResource;

fn main() -> io::Result<()> {
    #[cfg(windows)]
    {
        WindowsResource::new()
            .set("FileDescription", "Reverse-engineering sandbox")
            .set("ProductName", "malebolge")
            .set("OriginalFilename", "malebolge.exe")
            .set("LegalCopyright", "Copyright (c) 2023, Valaphee.")
            .set("CompanyName", "Valaphee")
            .set("InternalName", "malebolge.exe")
            .set_icon("malebolge.ico")
            .compile()?;
    }
    Ok(())
}
