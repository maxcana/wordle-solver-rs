use std::{collections::HashMap, fs, io::{self, Error}, time, usize};

// design:
// enter guess and result (eg. "crane _gy__")
// eliminate words that don't match the result
// try all possible guesses and find the best one
// the best one is the one that eliminates the most words on average
// average is calculated trying every secret word on the list

// implementation:
// for every guess
// for every secret word
// calculate words eliminated
// calculate avg words eliminated
// we'll validate possible secrets based on info using bitmaps

// MARK: main

fn load_words(file_path: &str) -> Vec<String> {
    let content: Result<String, Error> = fs::read_to_string(file_path);
    match content {
        Ok(s) => {
            let sm: String = s.trim().to_ascii_lowercase();
            // watch out for CRLF line endings, use .trim() each line
            let set: Vec<String> = sm.split("\n").map(|line: &str| line.trim().to_string()).collect();
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
fn encode_word(input: &str) -> [u8; 5] {
    input.to_ascii_lowercase().as_bytes().iter().map(|byte| *byte - 97u8).collect::<Vec<u8>>().try_into().unwrap() // try_into doesnt take ::<> generic params. not clear how it interprets it (something with the trait), but cool.
}
fn decode_word(input: &[u8; 5]) -> String {
    String::from_utf8(input.map(|byte| byte + 97u8).to_vec()).unwrap()
}
fn main() {
    // let inp: String = input("Enter guess and result (eg. \"crane _gy__\"").trim().to_lowercase();
    // let vec: Vec<&str> = inp.split(" ").collect();

    // let word: &str = vec[0];
    // let guess: &str = vec[1];

    // note: according to this script, aahed eliminates 12567.58 words on avg??  now 12338.69 on avg.
    // vs in python script aahed eliminates 13636.37 words on avg.

    let PG: Vec<[u8; 5]> = load_words("wordle_words.txt").iter().map(|str: &String| encode_word(str)).collect();
    let PS = PG.clone();
    let bitmap_cache: HashMap<[u8; 5], [u128; 3]> = PS.iter().map(|&word| word).zip(PS.iter().map(|&word| build_bitmap(word))).collect(); // absolute cinema

    // println!("bitmap for ps {:?}", PS[0]);
    // print_bitmap(&bitmap_cache[&PS[0]]);
    // panic!();

    let mut GS: HashMap<[u8; 5], f32> = HashMap::new();

    let start_inst = time::Instant::now();
    for pg in &PG {
        let mut total_elim: u32 = 0;

        for ps in &PS {
            let colors: [u8; 5] = get_colors(pg, ps);
            
            let mut bitmask: [u128; 3] = [0,0,0]; // 1 bitmap is 26 * 11 bits = 282 bits
            let mut minimum_of_ltr: [u8; 26] = [0; 26];
            let mut ltrs_with_maximum: [bool; 26] = [false; 26];
            
            // encode "positions" section of bitmask, while setting up minimums and maximums for "count" section
            for i in 0..5usize {
                let (letter, color) = (pg[i], colors[i]);

                match color {
                    0 => { //gray
                        write_bit(&mut bitmask, letter, i);
                        ltrs_with_maximum[pg[i] as usize] = true;
                    }
                    1 => { //yellow
                        write_bit(&mut bitmask, letter, i);
                        minimum_of_ltr[pg[i] as usize] += 1;
                    }
                    2 => { //green
                        write_row_excluding_bit(&mut bitmask, letter, i);
                        minimum_of_ltr[pg[i] as usize] += 1;
                    }
                    _ => {panic!()}
                }  
            }
            
            // encode "count" section of bitmask
            for i in 0..5usize {
                let ltr = pg[i] as usize;
                let min = minimum_of_ltr[ltr] as usize;
                
                // what quantities of this letter are disallowed? lets encode it
                for count in 0..6 { 
                    if ! if ltrs_with_maximum[ltr] {min == count} else {min <= count} {
                        // count section is from row 5 - row 10
                        write_bit(&mut bitmask, ltr as u8, count + 5)
                    };
                }
            }
            
            // if(start_inst.elapsed().as_millis() > 5000){
            //     println!("bitmask for guess {:?} with colors {:?} against secret {:?}", pg, colors, ps);
            //     print_bitmap(&bitmask);
            //     panic!();
            // }

            // bitmask is finished. lets compare it against every possible secret.

            let mut elim: u32 = 0;
            for (ps, bitmap) in &bitmap_cache {
                if !bitmaps_match(&bitmask, bitmap) {
                    elim += 1;
                }
            }
            total_elim += elim;
        }
        let avg_elim: f32 = total_elim as f32 / PS.len() as f32;
        println!("guess {} eliminates {:.2} words on average", decode_word(pg).to_ascii_uppercase(), avg_elim);
        GS.insert(*pg, avg_elim);

        // time estimates
        let amount_finished: f64 = GS.len() as f64 / PG.len() as f64;
        let elapsed: f64 = start_inst.elapsed().as_secs_f64();
        println!("estimated time remaining: {:.1}s", elapsed / amount_finished);

    }

}

// MARK: bit stuff

// what we are gonna do is encode the INFORMATION (guess & colors) with the 5 RULES so we can compare it with a bitwise AND to every POSSIBLE SECRET.
// this way we only calculate the rules [1 times] while encoding, and do bitwise AND [14855 times] instead of calculating the rules [14855 times]
// this is Ingenious

// dogma function // (we design our algorithm based on this being true)
/// #### check whether a possible secret is valid based on encoded information
/// bitmask: INFORMATION (encoded guess and colors). bitmap: possible secret to check validity of
fn bitmaps_match(bitmask: &[u128; 3], bitmap: &[u128; 3]) -> bool {
    bitmask.iter().zip(bitmap.iter()).all(|(&b, &m)| b & m == 0)
}

fn build_bitmap(ps: [u8; 5]) -> [u128; 3] {
    let mut bitmap: [u128; 3] = [0,0,0];
    
    let mut ltr_count: [u8; 26] = [0; 26];

    // encode "letter" section + counts prep
    for (i,&ltr) in ps.iter().enumerate() {
        write_bit(&mut bitmap, ltr, i);
        ltr_count[ltr as usize] += 1;
    }

    // encode "counts" section
    for ltr in ps {
        write_bit(&mut bitmap, ltr, 5 + ltr_count[ltr as usize] as usize);
    }

    return bitmap;
}

fn print_bitmap(bits: &[u128; 3]) {
    for r in 0..11 {
        let mut row = String::with_capacity(11);

        for c in 0..26 {
            let bit_index = r * 26 + c;
            row.push(if bits[bit_index / 128] & 1 << (127 - (bit_index % 128)) == 0 {'0'} else {'1'}); // fix this printing. its backwards. use 127 -!!
        }

        println!("{:0>4} {}", if r < 5 {("POS".to_string() + &r.to_string())} else if r <= 10 {("CNT".to_string() + &(r-5).to_string())} else {("EXT".to_string() + &(r-10).to_string())}, row);
    }
}

/// make a bit at position bitnum become 1
fn write_raw_bit(map: &mut[u128; 3], bitnum: usize){ map[bitnum / 128] = map[bitnum / 128] | (1u128 << (127 - (bitnum % 128))) }
/// make a bit at position bitnum become 0
fn delete_raw_bit(map: &mut[u128; 3], bitnum: usize){ map[bitnum / 128] = map[bitnum / 128] & !(1u128 << (127 -(bitnum % 128))) }

/// #### write a solid segment of 1 bits into a bitmap or bitmask, using left (inclusive) and right (inclusive) bound parameters
/// basically a workaround for not having a unsigned bigint. which is why this code looks intimidating, its just trying to make the array work as one biguint.
fn write_raw_bit_sequence(map: &mut[u128; 3], left: usize, right: usize){
    let leftmost_seg = left / 128;
    let rightmost_seg = right / 128;

    if leftmost_seg == rightmost_seg {
        // the write only affects one segment. lets actually write.
        let seg = left / 128;
        map[seg] = map[seg] | ((1u128 << (right-left+1)) - 1) << (128 - ((right % 128) + 1))
        // ex: left = 2, right = 5. 
        // 1 << (4) - 1 = 10000 - 1 = 1111. 1111 >> (left) = 001111. 001111 << (128 - (6 == right-left+1+left == right+1)) == 00111100000000000...
    } else {
        // the write affects multiple indices. recursive; calls this function for each segment affected.
        for i in leftmost_seg..=rightmost_seg {
            let seg_start = 128 * i;
            let seg_end = 128 * (i + 1) - 1;

            if i == leftmost_seg {
                write_raw_bit_sequence(map, left, seg_end);
            } else if i == rightmost_seg {
                write_raw_bit_sequence(map, seg_start, right);
            } else {
                write_raw_bit_sequence(map, seg_start, seg_end);
            }
        }
    }
}


/// #### write a 1 bit into a bitmap or bitmask.
/// letter = col (0-25), row = row (0-10). row includes position section (0-4) and count section (5-10)
fn write_bit(map: &mut[u128; 3], letter: u8, row: usize){
    write_raw_bit(map, 26 * row + letter as usize);
}

/// write an entire row of 1 bits into a bitmap or bitmask, except, leave a single column(letter) on the row as a 0.
fn write_row_excluding_bit(map: &mut[u128; 3], letter_excl: u8, row: usize){
    // make all the bits within bitnum_left and bitnum_right become 1
    write_raw_bit_sequence(map, 26 * row, 26 * (row+1)-1);

    // make the bit at bitnum become 0
    delete_raw_bit(map, 26 * row + letter_excl as usize);
}

// MARK: colors

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