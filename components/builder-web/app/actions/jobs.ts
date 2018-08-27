// Copyright (c) 2016-2017 Chef Software Inc. and/or applicable contributors
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

import * as depotApi from '../client/depot-api';
import { BuilderApiClient, ErrorCode } from '../client/builder-api';
import { addNotification } from './notifications';
import { DANGER, SUCCESS } from './notifications';

export const CLEAR_JOB = 'CLEAR_JOB';
export const CLEAR_JOB_LOG = 'CLEAR_JOB_LOG';
export const CLEAR_JOBS = 'CLEAR_JOBS';
export const POPULATE_JOB = 'POPULATE_JOB';
export const POPULATE_JOBS = 'POPULATE_JOBS';
export const POPULATE_JOB_LOG = 'POPULATE_JOB_LOG';
export const SET_JOB_LOADING = 'SET_JOB_LOADING';
export const SET_JOBS_LOADING = 'SET_JOBS_LOADING';
export const SET_JOB_LOG_LOADING = 'SET_JOB_LOG_LOADING';
export const SET_JOB_LOG_NOT_FOUND = 'SET_JOB_LOG_NOT_FOUND';
export const STREAM_JOB_LOG = 'STREAM_JOB_LOG';

export function clearJob() {
  return {
    type: CLEAR_JOB
  };
}

export function clearJobLog() {
  return {
    type: CLEAR_JOB_LOG
  };
}

export function clearJobs() {
  return {
    type: CLEAR_JOBS
  };
}

export function submitJob(origin: string, name: string, token: string) {
  return dispatch => {
    return depotApi.submitJob(origin, name, token)
      .then(response => {
        dispatch(addNotification({
          title: 'Job submitted',
          body: `A new job for ${origin}/${name} has been submitted.`,
          type: SUCCESS
        }));
        setTimeout(() => { dispatch(fetchJobs(origin, name, token)); }, 5000);
      })
      .catch(error => {
        dispatch(addNotification({
          title: 'Job request failed',
          body: `Reason: ${error}`,
          type: DANGER
        }));
      });
  };
}

export function fetchJobs(origin: string, name: string, token: string) {
  return dispatch => {
    dispatch(clearJobs);
    dispatch(setJobsLoading(true));

    new BuilderApiClient(token)
      .getJobs(origin, name)
      .then(data => {
        dispatch(populateJobs(data));
        dispatch(setJobsLoading(false));
      })
      .catch(error => {
        dispatch(populateJobs(null, error));
        dispatch(setJobsLoading(false));
      });
  };
}

export function fetchJob(id: string, token: string) {
  return dispatch => {
    dispatch(clearJob());
    dispatch(setJobLoading(true));

    new BuilderApiClient(token)
      .getJob(id)
      .then(data => {
        dispatch(populateJob(data));
        dispatch(setJobLoading(false));
      })
      .catch(error => {
        dispatch(populateJob(null, error));
        dispatch(setJobLoading(false));
      });
  };
}

export function fetchJobLog(id: string, token: string, start = 0) {
  return (dispatch, getState) => {
    dispatch(setJobLogLoading(true));

    if (start === 0) {
      dispatch(clearJobLog());
    }

    new BuilderApiClient(token)
      .getJobLog(id, start)
      .then(data => {
        dispatch(setJobLogLoading(false));
        dispatch(setJobLogNotFound(false));
        dispatch(populateJobLog(data));

        let complete = data['is_complete'];

        if (complete && data['start'] !== 0) {
          doAfter(5000, fetchJob(id, token));
        }
        else if (!complete && getState().jobs.selected.stream) {
          doAfter(2000, fetchJobLog(id, token, data['stop']));
        }
      })
      .catch(error => {
        dispatch(setJobLogLoading(false));
        dispatch(setJobLogNotFound(true));
        dispatch(populateJobLog(null, error));

        if (error.code === ErrorCode.NotFound  && getState().jobs.selected.stream) {
          doAfter(5000, fetchJob(id, token));
          doAfter(5000, fetchJobLog(id, token));
        }
      });

    function doAfter(delay: number, f: Function) {
      setTimeout(() => dispatch(f), delay);
    }
  };
}

function populateJob(data, error = undefined) {
  return {
    type: POPULATE_JOB,
    payload: data,
    error: error
  };
}

function populateJobs(data, error = undefined) {
  return {
    type: POPULATE_JOBS,
    payload: data ? data.data : undefined,
    error: error
  };
}

function populateJobLog(data, error = undefined) {
  return {
    type: POPULATE_JOB_LOG,
    payload: data,
    error: error
  };
}

function setJobLoading(loading: boolean) {
  return {
    type: SET_JOB_LOADING,
    payload: loading
  };
}

function setJobsLoading(loading: boolean) {
  return {
    type: SET_JOBS_LOADING,
    payload: loading
  };
}

function setJobLogLoading(loading: boolean) {
  return {
    type: SET_JOB_LOG_LOADING,
    payload: loading
  };
}

function setJobLogNotFound(notFound: boolean) {
  return {
    type: SET_JOB_LOG_NOT_FOUND,
    payload: notFound
  };
}

export function streamJobLog(setting) {
  return {
    type: STREAM_JOB_LOG,
    payload: setting
  };
}
