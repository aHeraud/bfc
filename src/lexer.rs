use std::str::CharIndices;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token {
    LoopStart,
    LoopEnd,
    Inc,
    Dec,
    IncPtr,
    DecPtr,
    Print,
    Read
}

#[derive(Debug)]
pub enum LexerError {}

pub type Spanned<Tok, Loc, Error> = Result<(Loc, Tok, Loc), Error>;

pub struct Lexer<'input> {
    _source: &'input str,
    chars: CharIndices<'input>
}

impl<'input> Lexer<'input> {
    pub fn new(_source: &'input str) -> Lexer<'input> {
        Lexer {
            _source,
            chars: _source.char_indices()
        }
    }
}

impl<'input> Iterator for Lexer<'input> {
    type Item = Spanned<Token, usize, LexerError>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.chars.next() {
                Some((index, '[')) => return Some(Ok((index, Token::LoopStart, index + 1))),
                Some((index, ']')) => return Some(Ok((index, Token::LoopEnd, index + 1))),
                Some((index, '+')) => return Some(Ok((index, Token::Inc, index + 1))),
                Some((index, '-')) => return Some(Ok((index, Token::Dec, index + 1))),
                Some((index, '>')) => return Some(Ok((index, Token::IncPtr, index + 1))),
                Some((index, '<')) => return Some(Ok((index, Token::DecPtr, index + 1))),
                Some((index, '.')) => return Some(Ok((index, Token::Print, index + 1))),
                Some((index, ',')) => return Some(Ok((index, Token::Read, index + 1))),
                None => return None,
                _ => continue
            }
        }
    }
}
