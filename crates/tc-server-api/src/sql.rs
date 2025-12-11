pub mod accounts {
    pub const ACCOUNT_EXISTS_BY_USERNAME: &str =
        "SELECT EXISTS(SELECT 1 FROM account WHERE username=$1);";
    pub const ACCOUNT_CREATE: &str = "INSERT INTO account(username, salt, verifier, reg_mail, email, joindate) VALUES($1, $2, $3, $4, $5, CURRENT_TIMESTAMP);";
    pub const ACCOUNT_INIT_REALM_CHARACTERS: &str = "
        INSERT INTO realmcharacters (realm_id, acct_id, num_chars)
        SELECT realmlist.id, account.id, 0
        FROM realmlist, account
        LEFT JOIN realmcharacters ON acct_id = account.id
        WHERE acct_id IS NULL
    ";
}
