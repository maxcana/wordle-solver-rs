use std::{collections::HashSet, fs, io, io::Error};

// design:
// enter guess and result (eg. "crane _gy__")
// eliminate words that don't match the result
// try all possible guesses and find the best one
// the best one is the one that eliminates the most words on average
// average is calculated by making the secret word every word on the list

// implementation:
// enter guess
// for every guess
// for every secret word
// calculate word eliminated
// calculate average word eliminated


// MARK: Bit stuff

// what we are gonna do is encode the INFORMATION (guess & colors) with the 5 RULES so we can compare it with a bitwise AND to every POSSIBLE SECRET.
// this way we only calculate the rules [1 times] while encoding, and do bitwise AND [2309 times] instead of calculating the rules [2309 times]
// this is Ingenious

// dogma function
// bitmask = INFORMATION, bitmap = POSSIBLE SECRET

fn load_words(file_path: &str) -> HashSet<String> {
    let content: Result<String, Error> = fs::read_to_string(file_path);
    match content {
        Ok(s) => {
            let sm: String = s.trim().to_lowercase();
            // map converts borrowed strings into new data
            let set: HashSet<String> = sm.split("\n").map(|line: &str| line.to_string()).collect();
            set
        },
        Err(e) => {panic!("ur mega cooked buddy")}
    }
}

fn input(query: &str) -> String{
    println!("{query}");
    let mut user_input: String = String::new();
    io::stdin().read_line(&mut user_input).expect("Failed to read user input");
    user_input
}

fn main() {
    let inp: String = input("Enter guess and result (eg. \"crane _gy__\"").trim().to_lowercase();
    let vec: Vec<&str> = inp.split(" ").collect();

    let word: &str = vec[0];
    let guess: &str = vec[1];

    let PG = load_words("wordle_words.txt");
    let PS = load_words("wordle_words.txt");

    for pg in &PG {
        for ps in &PS {
            // 1 bitmap is 26 * 11 bits = 282 bits
            let bitmask: [u128; 3] = [0,0,0];
        }
    }
    

}

/// Write a 1 bit into a bitmap or bitmask.
/// letter = col (0-25), pos = row (0-10).
/// pos includes position section (0-4) and count section (5-10)
fn write_bit(map: &mut[u128; 3], letter: usize, pos: usize){
    let bitnum: usize = 26 * pos + letter;
    map[bitnum / 128] = map[bitnum / 128] | ((1 as u128) << (bitnum % 128))
}
