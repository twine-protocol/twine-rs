
CREATE TABLE IF NOT EXISTS Strands (
  id BIGINT UNSIGNED PRIMARY KEY NOT NULL AUTO_INCREMENT,
  -- Cid bytes (2x varint (9) + 512bit hash (64)) = 18 + 64 = 82
  cid VARBINARY(82) UNIQUE NOT NULL,
  spec TEXT NOT NULL,
  data BLOB NOT NULL
);

CREATE INDEX idx_strands_cid ON Strands (cid);

CREATE TABLE IF NOT EXISTS Tixels (
  cid VARBINARY(82) UNIQUE NOT NULL,
  strand BIGINT UNSIGNED NOT NULL,
  idx BIGINT UNSIGNED NOT NULL,
  data BLOB NOT NULL,

  -- Keys
  PRIMARY KEY (strand, idx),
  FOREIGN KEY (strand) REFERENCES Strands(id) ON DELETE CASCADE
);

CREATE INDEX idx_tixels_cid ON Tixels (cid);
