-- Drop the generated company column: company is computed from raw JSON
-- in Rust (Data::company) and in queries via json_extract. Adding a
-- platform no longer requires regenerating this column.
ALTER TABLE jobs DROP COLUMN company;
