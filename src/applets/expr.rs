use crate::core::Applet;

pub struct ExprApplet;

impl Applet for ExprApplet {
    fn name(&self) -> &'static str {
        "expr"
    }

    fn description(&self) -> &'static str {
        "Evaluate expressions and print result"
    }

    fn run(&self, args: &[String]) -> Result<i32, Box<dyn std::error::Error>> {
        if args.is_empty() {
            return Ok(1);
        }

        let mut pos = 0;
        match parse_or_expr(args, &mut pos) {
            Ok(val) => {
                if pos < args.len() {
                    eprintln!("expr: syntax error");
                    return Ok(2);
                }
                println!("{}", val);
                if val.is_null_or_zero() {
                    Ok(1)
                } else {
                    Ok(0)
                }
            }
            Err(_) => {
                eprintln!("expr: syntax error");
                Ok(2)
            }
        }
    }

    fn help(&self) {
        println!("Usage: expr EXPRESSION");
        println!();
        println!("Evaluate expressions and print the result.");
        println!();
        println!("Arithmetic: + - * / %");
        println!("Comparison: = != < <= > >=");
        println!("Logical:    | (or) & (and)");
        println!("String:     length STRING");
        println!("            substr STRING POS LENGTH");
        println!();
        println!("Exit status:");
        println!("  0 if result is non-null and non-zero");
        println!("  1 if result is null or zero");
        println!("  2 on syntax error");
        println!();
        println!("Examples:");
        println!("  expr 1 + 2        # => 3");
        println!("  expr 5 '*' 3      # => 15 (quote * to avoid shell glob)");
        println!("  expr length hello # => 5");
    }
}

#[derive(Debug, Clone, PartialEq)]
enum Value {
    Integer(i64),
    Str(String),
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Integer(n) => write!(f, "{}", n),
            Value::Str(s) => write!(f, "{}", s),
        }
    }
}

impl Value {
    fn as_string(&self) -> String {
        match self {
            Value::Integer(n) => n.to_string(),
            Value::Str(s) => s.clone(),
        }
    }

    fn as_integer(&self) -> Result<i64, ()> {
        match self {
            Value::Integer(n) => Ok(*n),
            Value::Str(s) => s.parse::<i64>().map_err(|_| ()),
        }
    }

    fn is_null_or_zero(&self) -> bool {
        match self {
            Value::Integer(n) => *n == 0,
            Value::Str(s) => s.is_empty() || s == "0",
        }
    }
}

fn parse_or_expr(args: &[String], pos: &mut usize) -> Result<Value, ()> {
    let mut left = parse_and_expr(args, pos)?;

    while *pos < args.len() && args[*pos] == "|" {
        *pos += 1;
        let right = parse_and_expr(args, pos)?;
        if left.is_null_or_zero() {
            left = right;
        }
    }

    Ok(left)
}

fn parse_and_expr(args: &[String], pos: &mut usize) -> Result<Value, ()> {
    let mut left = parse_comparison(args, pos)?;

    while *pos < args.len() && args[*pos] == "&" {
        *pos += 1;
        let right = parse_comparison(args, pos)?;
        if left.is_null_or_zero() || right.is_null_or_zero() {
            left = Value::Integer(0);
        }
    }

    Ok(left)
}

fn parse_comparison(args: &[String], pos: &mut usize) -> Result<Value, ()> {
    let left = parse_additive(args, pos)?;

    if *pos >= args.len() {
        return Ok(left);
    }

    let op = &args[*pos];
    match op.as_str() {
        "=" | "!=" | "<" | "<=" | ">" | ">=" => {
            *pos += 1;
            let right = parse_additive(args, pos)?;
            let l_str = left.as_string();
            let r_str = right.as_string();

            let result = if let (Ok(l_num), Ok(r_num)) = (left.as_integer(), right.as_integer()) {
                match op.as_str() {
                    "=" => l_num == r_num,
                    "!=" => l_num != r_num,
                    "<" => l_num < r_num,
                    "<=" => l_num <= r_num,
                    ">" => l_num > r_num,
                    ">=" => l_num >= r_num,
                    _ => unreachable!(),
                }
            } else {
                match op.as_str() {
                    "=" => l_str == r_str,
                    "!=" => l_str != r_str,
                    "<" => l_str < r_str,
                    "<=" => l_str <= r_str,
                    ">" => l_str > r_str,
                    ">=" => l_str >= r_str,
                    _ => unreachable!(),
                }
            };

            Ok(Value::Integer(if result { 1 } else { 0 }))
        }
        _ => Ok(left),
    }
}

fn parse_additive(args: &[String], pos: &mut usize) -> Result<Value, ()> {
    let mut left = parse_multiplicative(args, pos)?;

    while *pos < args.len() && (args[*pos] == "+" || args[*pos] == "-") {
        let op = args[*pos].clone();
        *pos += 1;
        let right = parse_multiplicative(args, pos)?;
        let l = left.as_integer()?;
        let r = right.as_integer()?;
        left = Value::Integer(match op.as_str() {
            "+" => l.checked_add(r).ok_or(())?,
            "-" => l.checked_sub(r).ok_or(())?,
            _ => unreachable!(),
        });
    }

    Ok(left)
}

fn parse_multiplicative(args: &[String], pos: &mut usize) -> Result<Value, ()> {
    let mut left = parse_unary(args, pos)?;

    while *pos < args.len() && (args[*pos] == "*" || args[*pos] == "/" || args[*pos] == "%") {
        let op = args[*pos].clone();
        *pos += 1;
        let right = parse_unary(args, pos)?;
        let l = left.as_integer()?;
        let r = right.as_integer()?;
        left = Value::Integer(match op.as_str() {
            "*" => l.checked_mul(r).ok_or(())?,
            "/" => l.checked_div(r).ok_or(())?,
            "%" => l.checked_rem(r).ok_or(())?,
            _ => unreachable!(),
        });
    }

    Ok(left)
}

fn parse_unary(args: &[String], pos: &mut usize) -> Result<Value, ()> {
    if *pos >= args.len() {
        return Err(());
    }

    match args[*pos].as_str() {
        "length" => {
            *pos += 1;
            if *pos >= args.len() {
                return Ok(Value::Integer(0));
            }
            let s = &args[*pos];
            *pos += 1;
            Ok(Value::Integer(s.chars().count() as i64))
        }
        "substr" => {
            *pos += 1;
            if *pos + 2 >= args.len() {
                return Err(());
            }
            let s = &args[*pos];
            *pos += 1;
            let start = args[*pos].parse::<i64>().map_err(|_| ())?;
            *pos += 1;
            let len = args[*pos].parse::<i64>().map_err(|_| ())?;
            *pos += 1;

            if start <= 0 || len <= 0 {
                return Ok(Value::Str(String::new()));
            }

            let result = s
                .chars()
                .skip((start - 1) as usize)
                .take(len as usize)
                .collect();
            Ok(Value::Str(result))
        }
        _ => {
            let val = args[*pos].clone();
            *pos += 1;
            if let Ok(n) = val.parse::<i64>() {
                Ok(Value::Integer(n))
            } else {
                Ok(Value::Str(val))
            }
        }
    }
}
