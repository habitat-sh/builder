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

import * as actionTypes from '../actions/index';
import initialState from '../initial-state';

export default function builds(state = initialState['jobGroups'], action) {
  switch (action.type) {

    case actionTypes.POPULATE_JOB_GROUPS:
      return state.setIn(['visible'], action.payload);

    case actionTypes.POPULATE_JOB_GROUP:
      return state.setIn(['selected'], keyProjectsByState(action.payload));

    default:
      return state;
  }
}

function keyProjectsByState(payload) {
  payload.projects_by_state = payload.projects.reduce((acc, project) => {
    const state = project.state.toLowerCase();
    acc[state] = [...acc[state] || [], project];
    return acc;
  }, {});

  return payload;
}
