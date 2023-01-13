use heapless::Vec;
use nom::{
    branch::alt,
    bytes::complete::{tag, take_while},
    character::complete::{char, digit1},
    combinator::{map_res, opt},
    sequence::preceded,
    IResult,
};

pub fn parse(input: &str) -> IResult<&str, ()> {
    todo!()
}

fn parse_key(input: &str) -> IResult<&str, &str> {
    let (input, key) = alt((
        tag("A"),
        tag("B"),
        tag("C"),
        tag("D"),
        tag("E"),
        tag("F"),
        tag("G"),
        tag("RESET"),
    ))(input)?;
    Ok((input, key))
}

fn parse_symbol(input: &str) -> IResult<&str, Option<char>> {
    let (input, symbol) = opt(char('#'))(input)?;
    Ok((input, symbol))
}

fn parse_ocvate(input: &str) -> IResult<&str, u8> {
    let (input, ocvate) = map_res(digit1, str::parse)(input)?;
    Ok((input, ocvate))
}

fn parse_negative(input: &str) -> IResult<&str, &str> {
    let (input, neg) = tag("-")(input)?;
    Ok((input, neg))
}

fn parse_duration(input: &str) -> IResult<&str, u8> {
    let (input, ocvate) = map_res(digit1, str::parse)(input)?;
    Ok((input, ocvate))
}

fn sp(input: &str) -> IResult<&str, &str> {
    let chars = " \t\r\n";
    take_while(move |c| chars.contains(c))(input)
}
