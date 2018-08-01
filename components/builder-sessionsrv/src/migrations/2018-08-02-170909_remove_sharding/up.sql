/*
  This query is a report that's meant to be run manually. There's no code path (as of 2017-10-09) that calls it
*/
CREATE OR REPLACE FUNCTION account_creation_report_v2 (
  op_date timestamptz
) RETURNS SETOF accounts AS $$
    SELECT * FROM accounts WHERE created_at >= op_date;
$$ LANGUAGE SQL STABLE;
