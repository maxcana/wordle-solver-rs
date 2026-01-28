use std::{collections::HashSet, fs, io::{self, Error}, usize};

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
fn encode_word(input: &str) -> [u8; 5]{
    input.as_bytes().iter().map(|byte| *byte - 4u8).collect::<Vec<u8>>().try_into().unwrap() 
    // try_into doesnt take ::<> generic params. not clear why it interprets it (something with the trait), but cool.
}

fn main() {
    let inp: String = input("Enter guess and result (eg. \"crane _gy__\"").trim().to_lowercase();
    let vec: Vec<&str> = inp.split(" ").collect();

    let word: &str = vec[0];
    let guess: &str = vec[1];

    let PG: HashSet<[u8; 5]> = load_words("wordle_words.txt").iter().map(|str: &String| encode_word(str)).collect();
    let PS = PG.clone();

    for pg in &PG {
        for ps in &PS {
            let colors: [u8; 5] = get_colors(pg, ps);

            // 1 bitmap is 26 * 11 bits = 282 bits
            let mut bitmask: [u128; 3] = [0,0,0];

            for i in 0..5usize {
                let (letter, color) = (pg[i], colors[i]);

                match color {
                    0 => { //gray
                        write_bit(&mut bitmask, letter, i);
                    }
                    1 => { //yellow
                        write_bit(&mut bitmask, letter, i);
                    }
                    2 => { //green TODO
                        //write_bit(&mut bitmask, letter, pos);
                    }
                    _ => {panic!()}
                }
                
            }
        }
    }
    

}

/// write a 1 bit into a bitmap or bitmask.
/// letter = col (0-25), row = row (0-10).
/// row includes position section (0-4) and count section (5-10)
fn write_bit(map: &mut[u128; 3], letter: u8, row: usize){
    write_raw_bit(map, 26 * row + letter as usize);
}

/// make a bit at position bitnum become 1
fn write_raw_bit(map: &mut[u128; 3], bitnum: usize){ map[bitnum / 128] = map[bitnum / 128] | (1u128 << (bitnum % 128)) }
/// make a bit at position bitnum become 0
fn delete_raw_bit(map: &mut[u128; 3], bitnum: usize){ map[bitnum / 128] = map[bitnum / 128] & !(1u128 << (bitnum % 128)) }

/// write a solid segment of 1 bits into a bitmap or bitmask, using only left (inclusive) and right (inclusive) bound parameters
fn write_raw_bit_sequence(map: &mut[u128; 3], left: usize, right: usize){
    let first_arr_index = left / 128;
    let second_arr_index: usize = right / 128;
    if(first_arr_index == second_arr_index){
        // actually write
    } else {
        // TODO
        // write_raw_bit_sequence(map, left, final_bit_in_first_array);
        // write_raw_bit_sequence(map, first_bit_in_second_array, right);
    }
}

/// write an entire row of 1 bits into a bitmap or bitmask, except, leave a single column(letter) on the row as a 0.
fn write_row_excluding_single(map: &mut[u128; 3], letter_excl: u8, row: usize){
    let bitnum: usize = 26 * row + letter_excl as usize;
    let bitnum_left: usize = 26 * row;
    let bitnum_right: usize = 26 * (row+1)-1;

    // make all the bits within bitnum_left and bitnum_right become 1
    write_raw_bit_sequence(map, bitnum_left, bitnum_right);

    // make the bit at bitnum become 0
    delete_raw_bit(map, bitnum);
}


/// convert a guess and secret into colors (information for that guess).
fn get_colors(guess: &[u8; 5], secret: &[u8; 5]) -> [u8; 5]{
    let mut colors: [u8; 5] = [0,0,0,0,0];

    // used for populating colors with yellows at the end. tracks how many yellows are needed
    let mut how_many_yellows: [u8; 26] = [0; 26];

    // add greens
    for i in 0usize..5 {
        how_many_yellows[secret[i] as usize] += 1;

        if secret[i] == guess[i] {
            colors[i] = 2;
            how_many_yellows[secret[i] as usize] -= 1;
        }
    }

    // add yellows from left to right
    // the &ltr in the "pattern" means DESTRUCTURE, which automatically dereferences (copies) ltr.
    for (i, &ltr) in guess.iter().enumerate() {
        // if the letter is gray (and we have an excess of this letter from the secret) we make it yellow
        // we can make it yellow because were are sure it is not in this position already.
        if how_many_yellows[ltr as usize] > 0 && colors[i] == 0 {
            colors[i] = 1;
            how_many_yellows[secret[i] as usize] -= 1;
        }
    }

    return colors;
}