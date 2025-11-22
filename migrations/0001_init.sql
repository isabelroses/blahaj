CREATE TABLE IF NOT EXISTS game_data (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    date TEXT NOT NULL,
    rules TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS user_data (
    id INTEGER PRIMARY KEY,
    game_id INTEGER NOT NULL,
    success BOOLEAN NOT NULL,
    attempts INTEGER NOT NULL,
    attempted TEXT NOT NULL,
    FOREIGN KEY (game_id) REFERENCES game_data(id)
);
