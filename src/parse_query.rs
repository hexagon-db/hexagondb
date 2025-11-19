pub fn parse_query(query: String) -> Vec<String> {
    let mut parsed: Vec<String> = Vec::new();
    let mut in_quotes = false;
    let mut keyword = String::from("");
    for chr in query.chars() {
        if chr == '\''{
            in_quotes = !in_quotes;
            continue;
        }

        if chr == ' ' && !in_quotes{
            if keyword != ""{
                parsed.push(keyword);
                keyword = String::from("");
            }
        }
        else {
            keyword.push(chr);
        }
       
    }
    if keyword != ""{
        parsed.push(keyword);
    }

   parsed
}