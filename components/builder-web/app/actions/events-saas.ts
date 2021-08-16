// Copyright (c) 2021 Chef Software Inc. and/or applicable contributors
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

export const CLEAR_SAAS_EVENTS = 'CLEAR_SAAS_EVENTS';
export const SET_VISIBLE_SAAS_EVENTS = 'SET_VISIBLE_SAAS_EVENTS';
export const SET_SAAS_EVENTS_TOTAL_COUNT = 'SET_SAAS_EVENTS_TOTAL_COUNT';
export const SET_SAAS_EVENTS_NEXT_RANGE = 'SET_SAAS_EVENTS_NEXT_RANGE';
export const SET_SAAS_EVENTS_SEARCH_QUERY = 'SET__SAAS_EVENTS_SEARCH_QUERY';
export const SET_SAAS_EVENTS_DATE_FILTER = 'SET_SAAS_EVENTS_DATE_FILTER';

export function fetchSaasEvents(nextRange: number = 0, fromDate: string, toDate: string, query: string = '') {
  return dispatch => {
    if (nextRange === 0) {
      dispatch(clearEvents());
    }

    depotApi.getSaasEvents(nextRange, fromDate, toDate, query).then(response => {
      dispatch(setVisibleEvents(response['results']));
      dispatch(setEventsTotalCount(response['totalCount']));
      dispatch(setEventsNextRange(response['nextRange']));
    }).catch(error => {
      dispatch(setVisibleEvents(undefined, error));
    });
  };
}

function clearEvents() {
  return {
    type: CLEAR_SAAS_EVENTS,
  };
}

function setVisibleEvents(params, error = undefined) {
  return {
    type: SET_VISIBLE_SAAS_EVENTS,
    payload: params,
    error: error,
  };
}

function setEventsTotalCount(payload: number) {
  return {
    type: SET_SAAS_EVENTS_TOTAL_COUNT,
    payload,
  };
}

function setEventsNextRange(payload: number) {
  return {
    type: SET_SAAS_EVENTS_NEXT_RANGE,
    payload,
  };
}

export function setSaasEventsSearchQuery(payload: string) {
  return {
    type: SET_SAAS_EVENTS_SEARCH_QUERY,
    payload,
  };
}

export function setSaasEventsDateFilter(payload: any) {
  return {
    type: SET_SAAS_EVENTS_DATE_FILTER,
    payload,
  };
}