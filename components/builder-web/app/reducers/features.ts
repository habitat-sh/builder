// Copyright (c) 2016-2021 Chef Software Inc. and/or applicable contributors
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
import config from '../config';
import initialState from '../initial-state';

export default function builds(state = initialState['features'], action) {
  switch (action.type) {

    case actionTypes.LOAD_FEATURES:
      return state
        .setIn(['publishers', 'amazon'], !!config['enable_publisher_amazon'])
        .setIn(['publishers', 'azure'], !!config['enable_publisher_azure'])
        .setIn(['publishers', 'docker'], !!config['enable_publisher_docker'])
        .set('builder', !!config['enable_builder'])
        .set('events', !!config['enable_builder_events'])
        .set('saasEvents', !!config['enable_builder_events_saas']);
    default:
      return state;
  }
}
