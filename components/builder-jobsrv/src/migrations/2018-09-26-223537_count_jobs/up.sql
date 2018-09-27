CREATE OR REPLACE FUNCTION count_jobs_v1(in_job_state text) RETURNS bigint
    LANGUAGE plpgsql STABLE
    AS $$
  BEGIN
    RETURN COUNT(*) FROM jobs WHERE job_state = in_job_state;
  END
$$;