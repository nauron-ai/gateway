CREATE TYPE file_status AS ENUM ('pending', 'processing', 'success', 'failure');
CREATE TYPE file_origin AS ENUM ('upload', 'archive_entry');
