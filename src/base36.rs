const BASE36_CHARS: &[u8; 36] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ";

#[allow(dead_code)]
fn u32_to_base36(mut num: u32) -> String {
    let mut result = [b'0'; 4]; // Initialize a fixed-size array for the result

    for i in (0..4).rev() {
        result[i] = BASE36_CHARS[(num % 36) as usize];
        num /= 36;
    }

    // Convert the array of bytes to a String
    String::from_utf8(result.to_vec()).unwrap()
}

#[allow(dead_code)]
pub fn base36_to_u32(input: &str) -> Result<u32, &'static str> {
    // Reverse mapping for base36 characters
    let base36_map = {
        let mut map = [255; 256]; // 255 is an invalid index
        for (i, &c) in BASE36_CHARS.iter().enumerate() {
            map[c as usize] = i as u8;
        }
        map
    };

    let mut result: u32 = 0;

    for c in input.to_uppercase().chars() {
        // Convert the character to its base36 value
        let value = base36_map[c as usize];

        // Check if the character is valid
        if value == 255 {
            return Err("Invalid base36 character");
        }

        // Update the result by shifting and adding the new digit
        result = result * 36 + (value as u32);
    }

    Ok(result)
}
