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

// import * as depotApi from '../client/depot-api';
import { BuilderApiClient } from '../client/builder-api';
// import { addNotification } from './notifications';
// import { DANGER, SUCCESS } from './notifications';

export const POPULATE_JOB_GROUPS = 'POPULATE_JOB_GROUPS';
export const POPULATE_JOB_GROUP = 'POPULATE_JOB_GROUP';

export function fetchJobGroups(origin: string, token: string, limit: number = 10) {
  return dispatch => {
    new BuilderApiClient(token)
      .getJobGroups(origin, limit)
      .then(data => {
        dispatch(populateJobGroups(data));
      })
      .catch(error => {
        dispatch(populateJobGroups(null, error));
      });
  };
}

export function fetchJobGroup(id: string, token: string) {
  return dispatch => {
    new BuilderApiClient(token)
      .getJobGroup(id)
      .then(data => {
        dispatch(populateJobGroup(data));
      })
      .catch(error => {
        dispatch(populateJobGroup(null, error));
      });
  };
}

function populateJobGroups(data, error = undefined) {
  return {
    type: POPULATE_JOB_GROUPS,
    payload: data ? data : undefined,
    error
  };
}

function populateJobGroup(data, error = undefined) {
  return {
    type: POPULATE_JOB_GROUP,
    payload: data ? data : undefined,
    error
  };
}