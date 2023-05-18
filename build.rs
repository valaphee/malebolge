use std::io;

#[cfg(windows)]
use winres::WindowsResource;

fn main() -> io::Result<()> {
    #[cfg(windows)]
    {
        WindowsResource::new()
            .set("FileDescription", "Reverse-engineering sandbox")
            .set("ProductName", "amalgam")
            .set("OriginalFilename", "amalgam.exe")
            .set("LegalCopyright", "Copyright (c) 2023, Valaphee.")
            .set("CompanyName", "Valaphee")
            .set("InternalName", "amalgam.exe")
            .set_icon("amalgam.ico")
            .compile()?;
    }
    Ok(())
}
