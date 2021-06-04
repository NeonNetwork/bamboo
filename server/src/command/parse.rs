use super::{enums::StringType, Arg, Parser};
use common::math::Pos;
use std::{error::Error, fmt, str::FromStr};

#[derive(Debug, PartialEq)]
pub enum ParseError {
  /// Used when a literal does not match
  InvalidLiteral(String),
  /// Used when no children of the node matched
  NoChildren(Vec<ParseError>),
  /// Used when there are trailing characters after the command
  Trailing(String),
  /// Used when the string ends early.
  EOF(Parser),
  /// Used whenever a field does not match the given text.
  /// Values are the text, and the expected value.
  InvalidText(String, String),
  /// Used when a value is out of range
  Range(f64, Option<f64>, Option<f64>),
}

impl fmt::Display for ParseError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::InvalidLiteral(v) => write!(f, "invalid literal: {}", v),
      Self::NoChildren(errors) => {
        if errors.is_empty() {
          // No errors means print another error about no errors
          write!(f, "no errors in no children error (should never happen)")
        } else if errors.len() == 1 {
          // A single error should just be printed as that error
          write!(f, "{}", errors[0])
        } else {
          // Write all of the children in a row
          writeln!(f, "no children matched: [")?;
          for e in errors {
            write!(f, "  {}", e)?;
          }
          write!(f, "]")
        }
      }
      Self::Trailing(v) => write!(f, "trailing characters: {}", v),
      Self::EOF(v) => write!(f, "string ended early while parsing: {:?}", v),
      Self::InvalidText(text, expected) => {
        write!(f, "invalid text: {}. expected {}", text, expected)
      }
      Self::Range(v, min, max) => {
        if let (Some(min), Some(max)) = (min, max) {
          write!(f, "{} is out of range {}..{}", v, min, max)
        } else if let Some(min) = min {
          write!(f, "{} is less than min {}", v, min)
        } else if let Some(max) = max {
          write!(f, "{} is greater than max {}", v, max)
        } else {
          write!(f, "{} is out of range none (should never happen)", v)
        }
      }
    }
  }
}

impl Error for ParseError {}

fn parse_num<T>(
  text: &str,
  min: Option<T>,
  max: Option<T>,
  expected: &str,
) -> Result<(T, usize), ParseError>
where
  T: PartialOrd + FromStr + Into<f64> + Copy,
{
  let section = &text[..text.find(' ').unwrap_or(text.len())];
  match section.parse::<T>() {
    Ok(v) => {
      let mut invalid = false;
      if let Some(min) = min {
        if v < min {
          invalid = true;
        }
      }
      if let Some(max) = max {
        if v > max {
          invalid = true;
        }
      }
      if invalid {
        Err(ParseError::Range(v.into(), min.map(|v| v.into()), max.map(|v| v.into())))
      } else {
        Ok((v, section.len()))
      }
    }
    Err(_) => Err(ParseError::InvalidText(text.into(), expected.into())),
  }
}

impl Parser {
  pub fn parse(&self, text: &str) -> Result<(Arg, usize), ParseError> {
    match self {
      Self::Bool => {
        if text.starts_with("true") {
          Ok((Arg::Bool(true), 4))
        } else if text.starts_with("false") {
          Ok((Arg::Bool(false), 5))
        } else {
          Err(ParseError::InvalidText(text.into(), "true or false".into()))
        }
      }
      Self::Double { min, max } => {
        parse_num(text, *min, *max, "a double").map(|(num, len)| (Arg::Double(num), len))
      }
      Self::Float { min, max } => {
        parse_num(text, *min, *max, "a float").map(|(num, len)| (Arg::Float(num), len))
      }
      Self::Int { min, max } => {
        parse_num(text, *min, *max, "an int").map(|(num, len)| (Arg::Int(num), len))
      }
      Self::String(ty) => match ty {
        StringType::SingleWord => {
          if text.is_empty() {
            return Err(ParseError::EOF(self.clone()));
          }
          let section = &text[..text.find(' ').unwrap_or(text.len())];
          Ok((Arg::String(section.into()), section.len()))
        }
        StringType::QuotablePhrase => {
          if text.is_empty() {
            return Err(ParseError::EOF(self.clone()));
          }
          let mut iter = text.chars();
          if iter.next().unwrap() == '"' {
            let mut escaping = false;
            let mut index = 1;
            let mut out = String::new();
            for c in iter {
              if escaping {
                if c == '"' || c == '\\' {
                  out.push(c);
                } else {
                  return Err(ParseError::InvalidText(
                    text.into(),
                    "a valid escape character".into(),
                  ));
                }
                escaping = false;
              } else {
                if c == '"' {
                  break;
                } else if c == '\\' {
                  escaping = true;
                } else {
                  out.push(c);
                }
              }
              index += 1;
            }
            // Add 1 so that the ending quote is removed
            Ok((Arg::String(out), index + 1))
          } else {
            let mut index = 1;
            for c in iter {
              if c == ' ' {
                break;
              }
              index += 1;
            }
            Ok((Arg::String(text[0..index].into()), index))
          }
        }
        StringType::GreedyPhrase => Ok((Arg::String(text.into()), text.len())),
      },
      Self::Entity { single, players } => Ok((Arg::Int(5), 1)),
      Self::ScoreHolder { multiple } => Ok((Arg::Int(5), 1)),
      Self::GameProfile => Ok((Arg::Int(5), 1)),
      Self::BlockPos => {
        let sections: Vec<&str> = text.split(' ').collect();
        if sections.len() < 3 {
          return Err(ParseError::InvalidText(text.into(), "a block position".into()));
        }
        let x = sections[0].parse().map_err(|_| {
          ParseError::InvalidText(text.into(), "a valid block position coordinate".into())
        })?;
        let y = sections[1].parse().map_err(|_| {
          ParseError::InvalidText(text.into(), "a valid block position coordinate".into())
        })?;
        let z = sections[2].parse().map_err(|_| {
          ParseError::InvalidText(text.into(), "a valid block position coordinate".into())
        })?;
        Ok((
          Arg::BlockPos(Pos::new(x, y, z)),
          sections[0].len() + sections[1].len() + sections[2].len() + 2,
        ))
      }
      Self::ColumnPos => Ok((Arg::Int(5), 1)),
      Self::Vec3 => Ok((Arg::Int(5), 1)),
      Self::Vec2 => Ok((Arg::Int(5), 1)),
      Self::BlockState => Ok((Arg::Int(5), 1)),
      Self::BlockPredicate => Ok((Arg::Int(5), 1)),
      Self::ItemStack => Ok((Arg::Int(5), 1)),
      Self::ItemPredicate => Ok((Arg::Int(5), 1)),
      Self::Color => Ok((Arg::Int(5), 1)),
      Self::Component => Ok((Arg::Int(5), 1)),
      Self::Message => Ok((Arg::Int(5), 1)),
      Self::Nbt => Ok((Arg::Int(5), 1)),
      Self::NbtPath => Ok((Arg::Int(5), 1)),
      Self::Objective => Ok((Arg::Int(5), 1)),
      Self::ObjectiveCriteria => Ok((Arg::Int(5), 1)),
      Self::Operation => Ok((Arg::Int(5), 1)),
      Self::Particle => Ok((Arg::Int(5), 1)),
      Self::Rotation => Ok((Arg::Int(5), 1)),
      Self::Angle => Ok((Arg::Int(5), 1)),
      Self::ScoreboardSlot => Ok((Arg::Int(5), 1)),
      Self::Swizzle => Ok((Arg::Int(5), 1)),
      Self::Team => Ok((Arg::Int(5), 1)),
      Self::ItemSlot => Ok((Arg::Int(5), 1)),
      Self::ResourceLocation => Ok((Arg::Int(5), 1)),
      Self::MobEffect => Ok((Arg::Int(5), 1)),
      Self::Function => Ok((Arg::Int(5), 1)),
      Self::EntityAnchor => Ok((Arg::Int(5), 1)),
      Self::Range { decimals: bool } => Ok((Arg::Int(5), 1)),
      Self::IntRange => Ok((Arg::Int(5), 1)),
      Self::FloatRange => Ok((Arg::Int(5), 1)),
      Self::ItemEnchantment => Ok((Arg::Int(5), 1)),
      Self::EntitySummon => Ok((Arg::Int(5), 1)),
      Self::Dimension => Ok((Arg::Int(5), 1)),
      Self::Uuid => Ok((Arg::Int(5), 1)),
      Self::NbtTag => Ok((Arg::Int(5), 1)),
      Self::NbtCompoundTag => Ok((Arg::Int(5), 1)),
      Self::Time => Ok((Arg::Int(5), 1)),
      Self::Modid => Ok((Arg::Int(5), 1)),
      Self::Enum => Ok((Arg::Int(5), 1)),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn parse_types() -> Result<(), ParseError> {
    assert_eq!(Parser::Bool.parse("true")?, (Arg::Bool(true), 4));
    assert_eq!(Parser::Bool.parse("false")?, (Arg::Bool(false), 5));

    assert_eq!(Parser::Double { min: None, max: None }.parse("5.3")?, (Arg::Double(5.3), 3));
    assert_eq!(Parser::Double { min: None, max: None }.parse("3.0000")?, (Arg::Double(3.0), 6));
    assert_eq!(
      Parser::Double { min: Some(1.0), max: None }.parse("-5"),
      Err(ParseError::Range(-5.0, Some(1.0), None))
    );

    assert_eq!(Parser::Float { min: None, max: None }.parse("5.3")?, (Arg::Float(5.3), 3));
    assert_eq!(Parser::Float { min: None, max: None }.parse("3.0000")?, (Arg::Float(3.0), 6));
    assert_eq!(
      Parser::Float { min: Some(1.0), max: None }.parse("-5"),
      Err(ParseError::Range(-5.0, Some(1.0), None))
    );

    assert_eq!(Parser::Int { min: None, max: None }.parse("5")?, (Arg::Int(5), 1));
    assert_eq!(Parser::Int { min: None, max: None }.parse("03")?, (Arg::Int(3), 2));
    assert_eq!(
      Parser::Int { min: None, max: None }.parse("3.2"),
      Err(ParseError::InvalidText("3.2".into(), "an int".into()))
    );
    assert_eq!(
      Parser::Int { min: Some(1), max: None }.parse("-5"),
      Err(ParseError::Range(-5.0, Some(1.0), None))
    );

    assert_eq!(
      Parser::String(StringType::SingleWord).parse("big gaming")?,
      (Arg::String("big".into()), 3)
    );
    assert_eq!(
      Parser::String(StringType::SingleWord).parse("word")?,
      (Arg::String("word".into()), 4)
    );
    assert_eq!(
      Parser::String(StringType::SingleWord).parse(""),
      Err(ParseError::EOF(Parser::String(StringType::SingleWord))),
    );
    assert_eq!(
      Parser::String(StringType::QuotablePhrase).parse("big gaming")?,
      (Arg::String("big".into()), 3)
    );
    assert_eq!(
      Parser::String(StringType::QuotablePhrase).parse("\"big gaming\" things")?,
      (Arg::String("big gaming".into()), 12) // 10 + 2 because of the quotes
    );
    assert_eq!(
      Parser::String(StringType::QuotablePhrase).parse(r#""big gam\"ing" things"#)?,
      (Arg::String(r#"big gam"ing"#.into()), 14) // 11 + 2 + 1 because of the quotes and \
    );
    assert_eq!(
      Parser::String(StringType::QuotablePhrase).parse(r#""big gam\\\"ing" things"#)?,
      (Arg::String(r#"big gam\"ing"#.into()), 16)
    );
    assert_eq!(
      Parser::String(StringType::QuotablePhrase).parse(r#""big gam\\"ing" things"#)?,
      (Arg::String(r#"big gam\"#.into()), 11)
    );
    assert_eq!(
      Parser::String(StringType::GreedyPhrase).parse(r#""big gam\\"ing" things"#)?,
      (Arg::String(r#""big gam\\"ing" things"#.into()), 22)
    );

    assert_eq!(
      Parser::BlockPos.parse("10 12"),
      Err(ParseError::InvalidText("10 12".into(), "a block position".into())),
    );
    assert_eq!(Parser::BlockPos.parse("10 12 15")?, (Arg::BlockPos(Pos::new(10, 12, 15)), 8));
    // Parser::Double { min, max } => {
    // Parser::Float { min, max } => (),
    // Parser::Int { min, max } => (),
    // Parser::String(StringType) => (),
    // Parser::Entity { single, players } => (),
    // Parser::ScoreHolder { multiple } => (),
    // Parser::GameProfile => (),
    // Parser::BlockPos => (),
    // Parser::ColumnPos => (),
    // Parser::Vec3 => (),
    // Parser::Vec2 => (),
    // Parser::BlockState => (),
    // Parser::BlockPredicate => (),
    // Parser::ItemStack => (),
    // Parser::ItemPredicate => (),
    // Parser::Color => (),
    // Parser::Component => (),
    // Parser::Message => (),
    // Parser::Nbt => (),
    // Parser::NbtPath => (),
    // Parser::Objective => (),
    // Parser::ObjectiveCriteria => (),
    // Parser::Operation => (),
    // Parser::Particle => (),
    // Parser::Rotation => (),
    // Parser::Angle => (),
    // Parser::ScoreboardSlot => (),
    // Parser::Swizzle => (),
    // Parser::Team => (),
    // Parser::ItemSlot => (),
    // Parser::ResourceLocation => (),
    // Parser::MobEffect => (),
    // Parser::Function => (),
    // Parser::EntityAnchor => (),
    // Parser::Range { decimals: bool } => (),
    // Parser::IntRange => (),
    // Parser::FloatRange => (),
    // Parser::ItemEnchantment => (),
    // Parser::EntitySummon => (),
    // Parser::Dimension => (),
    // Parser::Uuid => (),
    // Parser::NbtTag => (),
    // Parser::NbtCompoundTag => (),
    // Parser::Time => (),
    // Parser::Modid => (),
    // Parser::Enum => (),
    Ok(())
  }
}
