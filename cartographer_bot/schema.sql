CREATE TABLE IF NOT EXISTS Games (game_id INTEGER PRIMARY KEY);

CREATE TABLE IF NOT EXISTS Reservations (
    game_id INTEGER REFERENCES Games (game_id) ON DELETE CASCADE,
    user_id INTEGER,
    timestamp TEXT,
    tag VARCHAR(3),
    PRIMARY KEY (game_id, user_id)
);