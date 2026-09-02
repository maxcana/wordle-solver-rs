use std::{collections::HashMap, fs, io::{self, BufWriter, Error, Write}, sync::mpsc, thread::{self, JoinHandle}, time::{self, Instant}, usize};

static THREADS: usize = 12;
static CALCULATE_INITIALLY: bool = false;
static SHOW_TOP_X_GUESSES: usize = 5;
static PRINT_WORST_GUESS: bool = true;

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

// MARK: Helpers

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
fn clear_lines(n: usize) {
    let mut out = io::stdout();
    for _ in 0..n {
        write!(out, "\x1b[1A\r\x1b[2K").unwrap();
    }
    out.flush().unwrap();
}
fn input(query: &str) -> String{
    eprint!("{query}");
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
#[derive(Copy, Clone)]
struct GuessScore {
    total_elim: u32,
    avg_elim: f32,
}
impl GuessScore {
    fn print(self, pg: &[u8;5], counted: u32, total: u32, start_inst: &Instant){
        // all of this for "atomic" printing. so you dont see it print each line individually. it just clears, prints both, then renders
        let mut out = BufWriter::new(io::stdout().lock());
        for i in 0..2 { write!(out, "\x1b[1A\r\x1b[2K").unwrap(); }
        writeln!(out, "Guess {} eliminates {:.2} words on average", decode_word(pg).to_ascii_uppercase(), self.avg_elim).unwrap();
        let amount_finished: f64 = counted as f64 / total as f64;
        let elapsed: f64 = start_inst.elapsed().as_secs_f64();
        writeln!(out, "Progress: {:.2}%. Time remaining: {:.0}s. Counted: {} / {}", amount_finished * 100f64, elapsed / amount_finished, counted, total).unwrap();
        
        out.flush().unwrap();
    }
    fn handle_scored_guess(self, GS: &mut HashMap<[u8;5], GuessScore>, pg: &[u8; 5], counted: &mut u32, total: u32, start_inst: &Instant){
        self.print(pg, {*counted += 1; *counted}, total, &start_inst);
        GS.insert(*pg, self.clone());
    }
}
fn prepare_for_printing_guess_scores() {
    // so it clears these empty lines on the first iteration
    for i in 0..2 { println!() }
}

fn obtain_user_info_and_eliminate_PS(PS: &mut Vec<[u8; 5]>, BITMAP_CACHE: &HashMap<[u8; 5], [u128; 3]>) {
    while true {
        let inp: String = input("Enter guess and result (lares __g_y): ").trim().to_lowercase();
        if(inp.len() != 11 || inp.chars().nth(5).unwrap() != ' ' || !inp.chars().take(5).all(|c| c.is_ascii_alphabetic())) {
            println!("invalid input");
            continue
        }

        let vec: Vec<&str> = inp.split(" ").collect();
        let guess_inp: &str = vec[0];
        let colors_inp: &str = vec[1];
        
        let mut guess: [u8; 5] = [0; 5];
        for i in 0..5 {
            guess[i] = guess_inp.to_ascii_lowercase().chars().nth(i).unwrap() as u8 - 'a' as u8;
        }
        let mut colors: [u8; 5] = [0; 5];
        for i in 0..5 {
            colors[i] = match colors_inp.chars().nth(i).unwrap() {'_' => 0, 'y' => 1, 'g' => 2, _ => 0};
        }
        
        let elimination_info = build_bitmask(&guess, &colors);

        let old_PS_len = PS.len();
        let new_PS: Vec<[u8; 5]> = PS.iter().filter(|&ps| bitmaps_match(&elimination_info, &BITMAP_CACHE[ps])).cloned().collect();
        let new_PS_len = new_PS.len();
        if(new_PS_len == 0) {
            println!("that would eliminate every word, did you make a mistake?");
            continue
        }
        *PS = new_PS;

        println!("Filtered possible secrets: {} -> {} (-{})", old_PS_len, new_PS_len, old_PS_len - new_PS_len);
        break
    }
}


// MARK: Multithreading
fn main() {
    let PG: Vec<[u8; 5]> = load_words("wordle_words.txt").iter().map(|str: &String| encode_word(str)).collect();
    let mut PS: Vec<[u8; 5]> = PG.clone();
    let BITMAP_CACHE: HashMap<[u8; 5], [u128; 3]> = PS.iter().map(|&word| word).zip(PS.iter().map(|&word| build_bitmap(word))).collect(); // absolute cinema
    
    let mut PG_SPLITS: Vec<Vec<[u8; 5]>> = (0..THREADS).map(|_| Vec::with_capacity(PG.len()/THREADS+1)).collect();
    PG.iter().enumerate().for_each(|(i,item)| PG_SPLITS[i % THREADS].push(*item));

    let mut runs: usize = 0;
    while true {
        runs += 1;
        if runs > 1 || !CALCULATE_INITIALLY {
            // prompt user
            obtain_user_info_and_eliminate_PS(&mut PS, &BITMAP_CACHE);
        }

        let (tx, rx) = mpsc::channel::<([u8; 5], GuessScore)>();
        let mut threads = Vec::<JoinHandle<HashMap<[u8; 5], GuessScore>>>::new();
        let start_inst = time::Instant::now(); let mut counted: u32 = 0; let total: u32 = PG.len() as u32;
        println!("Starting {} threads...", THREADS);
        for i in 1..THREADS {
            // need to explicitly define new variables. it won't work if it captures PG_SPLITS or PS, etc. because the thread may start after they're dropped.
            let cap_tx = tx.clone();
            let cap_PG_SPLITS = PG_SPLITS[i].clone();
            let cap_PS = PS.clone();
            let cap_BITMAP_CACHE = BITMAP_CACHE.clone();
            threads.push(thread::spawn(move || solve(&cap_PG_SPLITS, &cap_PS, &cap_BITMAP_CACHE, |pg, guess_score| {
                cap_tx.send((*pg, guess_score)).unwrap();
            })));
        }
        
        prepare_for_printing_guess_scores();
        let mut GS: HashMap<[u8;5], GuessScore> = HashMap::with_capacity(14855);
        // use the main thread to solve, and check for the other threads' results
        solve(&PG_SPLITS[0], &PS, &BITMAP_CACHE, |pg, guess_score| {
            guess_score.handle_scored_guess(&mut GS, pg, &mut counted, total, &start_inst);
                while let Ok(s) = rx.try_recv() {
                    s.1.handle_scored_guess(&mut GS, &s.0, &mut counted, total, &start_inst);
                }
            }
        );
        // check every little while for new guess scores
        // while waiting for all other threads to complete
        while counted < total {
            let sc = rx.recv();
            match sc {
                Ok(s) => s.1.handle_scored_guess(&mut GS, &s.0, &mut counted, total, &start_inst),
                Err(_) => break
            }
        }

        let mut GS_VEC = GS.iter().collect::<Vec<(&[u8; 5], &GuessScore)>>();
        GS_VEC.sort_by(|a,b| (&b.1.total_elim).cmp(&a.1.total_elim));
        // print best guesses
        for i in 0..SHOW_TOP_X_GUESSES {
            println!("#{} Guess {} eliminates {:.2} words on average", (i+1), decode_word(GS_VEC[i].0).to_ascii_uppercase(), GS_VEC[i].1.avg_elim);
        }
        if PRINT_WORST_GUESS {
            println!("Worst Guess {} eliminates {:.2} words on average", decode_word(GS_VEC.last().unwrap().0).to_ascii_uppercase(), GS_VEC.last().unwrap().1.avg_elim);
        }
    }

}

// MARK: Solver
fn solve(PG: &Vec<[u8;5]>, PS: &Vec<[u8; 5]>, BITMAP_CACHE: &HashMap<[u8; 5], [u128; 3]>, mut on_guess_scored: impl FnMut(&[u8;5], GuessScore)) -> HashMap<[u8;5],GuessScore> {
    let mut GS: HashMap<[u8; 5], GuessScore> = HashMap::new();
    for pg in PG {
        let mut total_elim: u32 = 0;

        for ps in PS {
            let colors: [u8; 5] = get_colors(pg, ps);
            
            let bitmask = build_bitmask(pg, &colors);

            // bitmask is finished. lets compare it against every possible secret.

            let mut elim: u32 = 0;
            for (ps, bitmap) in BITMAP_CACHE {
                if !bitmaps_match(&bitmask, bitmap) {
                    elim += 1;
                }
            }
            total_elim += elim;
        }
        let avg_elim: f32 = if PS.len() == 0 { 0.0 } else { total_elim as f32 / PS.len() as f32 };
        on_guess_scored(pg, GuessScore { total_elim, avg_elim });
        GS.insert(*pg, GuessScore { total_elim, avg_elim });
    }
    GS
}

// MARK: bit stuff

// what we are gonna do is encode the INFORMATION (guess & colors) with the 5 RULES so we can compare it with a bitwise AND to every POSSIBLE SECRET.
// this way we only calculate the rules [1 times] while encoding, and do bitwise AND [14855 times] instead of calculating the rules [14855 times]
// this is Ingenious

/// #### check whether a possible secret is valid based on encoded information
/// bitmask: INFORMATION (encoded guess and colors). bitmap: possible secret to check validity of
fn bitmaps_match(bitmask: &[u128; 3], bitmap: &[u128; 3]) -> bool {
    bitmask.iter().zip(bitmap.iter()).all(|(&b, &m)| b & m == 0)
}

/// takes word and returns bitmap complementary to the information bitmask such that taking an AND can check validity of the word as a secret (see design image pls [red map on diagram])
fn build_bitmap(ps: [u8; 5]) -> [u128; 3] {
    let mut bitmap: [u128; 3] = [0,0,0];
    
    let mut ltr_count: [u8; 26] = [0; 26];

    // encode "letter" section + counts prep
    for (i,&ltr) in ps.iter().enumerate() {
        write_bit(&mut bitmap, ltr, i);
        ltr_count[ltr as usize] += 1;
    }

    // encode "counts" section
    for ltr in 0..26u8 {
        write_bit(&mut bitmap, ltr, 5 + ltr_count[ltr as usize] as usize);
    }

    return bitmap;
}

/// takes guess and colors and encodes that into an information bitmask
fn build_bitmask(pg: &[u8; 5], colors: &[u8; 5]) -> [u128; 3] {
    let mut bitmask: [u128; 3] = [0,0,0]; // 1 bitmap is 26 * 11 bits = 282 bits
    let mut minimum_of_ltr: [u8; 26] = [0; 26];
    let mut ltrs_with_maximum: [bool; 26] = [false; 26];
    
    // encode "positions" section of bitmask, while setting up minimums and maximums for "count" section
    for i in 0..5usize {
        let (letter, ltr, color) = (pg[i] as usize, pg[i], colors[i]);

        match color {
            0 => { //gray
                write_bit(&mut bitmask, ltr, i);
                ltrs_with_maximum[letter] = true;
            }
            1 => { //yellow
                write_bit(&mut bitmask, ltr, i);
                minimum_of_ltr[letter] += 1;
            }
            2 => { //green
                write_row_excluding_bit(&mut bitmask, ltr, i);
                minimum_of_ltr[letter] += 1;
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
    bitmask
}

fn print_bitmap(bits: &[u128; 3]) {
    for r in 0..11 {
        let mut row = String::with_capacity(11);

        for c in 0..26 {
            let bit_index = r * 26 + c;
            row.push(if bits[bit_index / 128] & 1 << (127 - (bit_index % 128)) == 0 {'0'} else {'1'});
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
            how_many_yellows[ltr as usize] -= 1;
        }
    }

    return colors;
}