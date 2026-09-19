mod lexer;

use lexer::Lexer; 

fn main() {
    let source = "let result = fib(10);";
    let mut lexer = Lexer::new(source); 

    loop {
        let token = lexer.next_token();
        println!("{:?}", token); 
        if token == lexer::token::Token::EOF {
            break; 
        }
    }
}
