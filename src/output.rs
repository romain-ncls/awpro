/// Print either a plain string or a JSON object (already serialised) depending on the `json` flag.
pub fn print(plain: &str, json_value: serde_json::Value, json: bool) {
    if json {
        println!("{}", json_value);
    } else {
        println!("{plain}");
    }
}
