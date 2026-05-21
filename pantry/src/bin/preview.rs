use std::io::{self, Write};

fn main() {
    let mut stdout = io::stdout();
    
    // Clear screen and move cursor to top-left (but stay on main screen)
    write!(stdout, "\x1b[2J\x1b[H").unwrap();
    stdout.flush().unwrap();
    
    // Top bar
    print!("\x1b[48;2;37;37;37m");
    print!("\x1b[38;2;128;128;128m main\x1b[2m src/components\x1b[0m");
    print!("\x1b[48;2;37;37;37m");
    print!("\x1b[38;2;128;128;128m                                    4 ✓  4.56%\x1b[0m");
    println!();
    
    // Empty line
    println!();
    
    // User message
    print!("\x1b[38;2;255;255;255m\x1b[1m❯ \x1b[0m");
    print!("\x1b[38;2;224;224;224mEdit the copy on this page\x1b[0m");
    println!();
    println!();
    
    // Thought
    print!("\x1b[2m\x1b[38;2;128;128;128m◆ \x1b[0m");
    print!("\x1b[38;2;128;128;128mThought for 2.5s\x1b[0m");
    println!();
    println!();
    
    // Edit
    print!("\x1b[38;2;128;128;128m◆ Edit \x1b[0m");
    print!("\x1b[38;2;247;140;108mfrontend/apps/website/src/app/(main)/cli/page.tsx\x1b[0m");
    println!();
    println!();
    
    // Code block
    print!("\x1b[38;2;128;128;128m\x1b[2m  770  \x1b[0m");
    print!("\x1b[38;2;92;207;230m<h3 className=\"\x1b[0m");
    print!("\x1b[38;2;126;231;135mtext-balance text-3xl font-semibold tracking-tight\x1b[0m");
    print!("\x1b[38;2;92;207;230m\">\x1b[0m");
    println!();
    
    print!("\x1b[38;2;128;128;128m\x1b[2m  771  \x1b[0m");
    print!("\x1b[38;2;224;224;224m    Better title\x1b[0m");
    println!();
    
    print!("\x1b[38;2;128;128;128m\x1b[2m  773  \x1b[0m");
    print!("\x1b[48;2;30;50;30m\x1b[38;2;126;231;135m    <p className=\"text-secondary mx-auto mt-4\">\x1b[0m");
    println!();
    
    print!("\x1b[38;2;128;128;128m\x1b[2m  774  \x1b[0m");
    print!("\x1b[48;2;30;50;30m\x1b[38;2;126;231;135m      A terminal-native experience...\x1b[0m");
    println!();
    println!();
    
    // Input bar
    print!("\x1b[48;2;30;30;30m");
    print!("┌──────────────────────────────────────────────────────────────────────────────┐\x1b[0m");
    println!();
    print!("\x1b[48;2;30;30;30m│ \x1b[0m");
    print!("\x1b[48;2;30;30;30m\x1b[1m\x1b[38;2;255;255;255m❯ \x1b[0m");
    print!("\x1b[48;2;30;30;30m\x1b[38;2;224;224;224m/btw\x1b[0m");
    print!("\x1b[48;2;30;30;30m                                     \x1b[2m\x1b[38;2;128;128;128mgrok-build-latest · always-approve\x1b[0m");
    print!("\x1b[48;2;30;30;30m │\x1b[0m");
    println!();
    print!("\x1b[48;2;30;30;30m");
    print!("└──────────────────────────────────────────────────────────────────────────────┘\x1b[0m");
    println!();
    println!();
    
    // Status bar
    print!("\x1b[38;2;128;128;128mEnter\x1b[0m");
    print!("\x1b[2m\x1b[38;2;128;128;128m send  |  \x1b[0m");
    print!("\x1b[38;2;128;128;128mShift-Tab\x1b[0m");
    print!("\x1b[2m\x1b[38;2;128;128;128m normal  |  \x1b[0m");
    print!("\x1b[38;2;128;128;128m^q\x1b[0m");
    print!("\x1b[2m\x1b[38;2;128;128;128m quit\x1b[0m");
    println!();
    
    // Reset
    print!("\x1b[0m");
    stdout.flush().unwrap();
    
    // Wait for user input before clearing
    println!();
    println!("Press Enter to exit...");
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
}
