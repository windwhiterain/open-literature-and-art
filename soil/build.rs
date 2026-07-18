fn main() {
    let url = option_env!("SOIL_PROJECT_URL")
        .unwrap_or("https://github.com/windwhiterain/open-literature-and-art.git");
    println!("cargo:rustc-env=SOIL_PROJECT_URL={}", url);
}
