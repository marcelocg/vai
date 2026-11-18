struct Account {
    owner: String,
    balance: i64,
}

impl Account {
    fn new(owner: String) -> Self {
        Account { owner, balance: 0 }
    }

    fn deposit(&mut self, amount: i64) {
        self.balance += amount;
    }

    fn status(&self) -> &'static str {
        match self.balance {
            0 => "empty",
            b if b < 0 => "overdrawn",
            _ => "ok",
        }
    }
}

fn main() {
    let mut accounts = vec![Account::new("ana".into()), Account::new("bia".into())];
    for acc in accounts.iter_mut() {
        acc.deposit(100);
    }
    let rich: Vec<&Account> = accounts
        .iter()
        .filter(|a| {
            let threshold = 50;
            a.balance > threshold
        })
        .collect();
    for a in &rich {
        println!("{} is {}", a.owner, a.status());
    }
}
