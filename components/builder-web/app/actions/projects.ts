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

import { BuilderApiClient } from '../client/builder-api';
import { addNotification } from './notifications';
import { DANGER, SUCCESS } from './notifications';
import { targets } from '../util';

export const CLEAR_PROJECTS = 'CLEAR_PROJECTS';
export const CLEAR_CURRENT_PROJECT = 'CLEAR_CURRENT_PROJECT';
export const CLEAR_CURRENT_PROJECT_INTEGRATION = 'CLEAR_CURRENT_PROJECT_SETTINGS';
export const DELETE_PROJECT = 'DELETE_PROJECT';
export const SET_CURRENT_PROJECT = 'SET_CURRENT_PROJECT';
export const SET_CURRENT_PROJECTS = 'SET_CURRENT_PROJECTS';
export const SET_CURRENT_PROJECT_INTEGRATION = 'SET_CURRENT_PROJECT_INTEGRATION';
export const SET_PROJECTS = 'SET_PROJECTS';

function clearProjects() {
  return {
    type: CLEAR_PROJECTS
  };
}

export function setProjectIntegrationSettings(origin: string, name: string, integration: string, settings: any, token: string) {
  return dispatch => {
    new BuilderApiClient(token).setProjectIntegrationSettings(origin, name, integration, settings)
      .then(response => {
        dispatch(addNotification({
          title: 'Integration settings saved',
          type: SUCCESS
        }));
      })
      .catch(error => {
        dispatch(addNotification({
          title: 'Failed to save integration settings',
          body: error.message,
          type: DANGER
        }));
      });
  };
}

export function setProjectVisibility(origin: string, name: string, setting: string, token: string) {
  return dispatch => {
    new BuilderApiClient(token).setProjectVisibility(origin, name, setting)
      .then(response => {
        dispatch(addNotification({
          title: 'Privacy settings saved',
          type: SUCCESS
        }));
      })
      .catch(error => {
        dispatch(addNotification({
          title: 'Failed to save privacy settings',
          body: error.message,
          type: DANGER
        }));
      });
  };
}

export function fetchProject(origin: string, name: string, target: string, token: string, alert: boolean) {
  return dispatch => {
    dispatch(clearCurrentProject());
    dispatch(fetchCurrentProjects(origin, name, token));

    new BuilderApiClient(token).getProject(origin, name, target)
      .then(response => {
        dispatch(setCurrentProject(response, null));
      })
      .catch((error) => {
        dispatch(setCurrentProject(null, error));
      });
  };
}

export function fetchProjects(origin: string, token: string) {
  return dispatch => {
    dispatch(clearProjects());
    dispatch(clearCurrentProject());

    new BuilderApiClient(token).getProjects(origin).then(response => {
      if (Array.isArray(response) && response.length > 0) {
        dispatch(setProjects(response));
      }
    });
  };
}

export function fetchProjectIntegration(origin: string, name: string, integration: string, token: string) {
  return dispatch => {
    new BuilderApiClient(token).getProjectIntegration(origin, name, integration)
      .then(response => {
        dispatch(setCurrentProjectIntegration({
          name: integration,
          settings: response
        }));
      })
      .catch(error => { });
  };
}

export function fetchCurrentProjects(origin: string, name: string, token: string) {
  return dispatch => {
    const fetchAll = targets.map(target => {
      return new BuilderApiClient(token)
        .getProject(origin, name, target.id)
        .catch(error => null);
    });

    Promise.all(fetchAll)
      .then(projects => dispatch(setCurrentProjects(projects)))
      .catch(error => dispatch(setCurrentProjects([])));
  };
}

export function deleteProjectIntegration(origin: string, name: string, integration: string, token: string) {
  return dispatch => {
    new BuilderApiClient(token).deleteProjectIntegration(origin, name, integration).then(response => {
      dispatch(addNotification({
        title: 'Integration settings deleted',
        type: SUCCESS
      }));
    }).catch(error => {
      dispatch(addNotification({
        title: 'Failed to delete integration settings',
        body: error.message,
        type: DANGER
      }));
    });
  };
}

function clearCurrentProject() {
  return {
    type: CLEAR_CURRENT_PROJECT
  };
}

export function setCurrentProject(project, error = undefined) {
  return {
    type: SET_CURRENT_PROJECT,
    payload: project,
    error: error
  };
}

export function setCurrentProjects(projects, error = undefined) {
  return {
    type: SET_CURRENT_PROJECTS,
    payload: projects,
    error: error
  };
}

function setCurrentProjectIntegration(settings) {
  return {
    type: SET_CURRENT_PROJECT_INTEGRATION,
    payload: settings
  };
}

function setProjects(projects) {
  return {
    type: SET_PROJECTS,
    payload: projects,
  };
}
