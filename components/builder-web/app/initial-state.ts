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

import { List, Record } from 'immutable';
import { BehaviorSubject } from 'rxjs';

import { Origin } from './records/Origin';
import { Package } from './records/Package';
import { Project } from './records/Project';
import { latestBase } from './util';

export default Record({
  app: Record({
    name: 'Habitat',
    currentYear: new Date().getFullYear(),
  })(),
  session: Record({
    token: undefined
  })(),
  gitHub: Record({
    installations: List(),
    repositories: List(),
    ui: Record({
      installations: Record({
        loading: false
      })(),
    repositories: Record({
        loading: false
      })()
    })()
  })(),
  jobGroups: Record({
    visible: List(),
    selected: Record({
      id: undefined,
      created_at: undefined,
      project_name: undefined,
      projects: List(),
      projects_by_state: Record({})(),
      state: undefined
    })()
  })(),
  jobs: Record({
    groups: List(),
    visible: List(),
    selected: Record({
      info: Record({
        id: undefined,
        owner_id: undefined,
        origin: undefined,
        name: undefined,
        version: undefined,
        release: undefined,
        state: undefined,
        build_start: undefined,
        build_stop: undefined,
        created_at: undefined
      })(),
      log: Record({
        start: undefined,
        stop: undefined,
        content: new BehaviorSubject([]),
        is_complete: undefined,
        stream: undefined
      })(),
      stream: false,
    })(),
    ui: Record({
      loading: false,
      selected: Record({
        info: Record({
          loading: false
        })(),
        log: Record({
          loading: false,
          notFound: false
        })()
      })()
    })()
  })(),
  features: Record({
    publishers: Record({
      amazon: false,
      azure: false,
      docker: false
    })(),
    builder: false,
    events: false,
    saasEvents: false,
    visibility: false,
    enableBase: false,
  })(),
  notifications: Record({
    all: List(),
  })(),
  oauth: Record({
    state: undefined,
    token: undefined,
    provider: Record({
      type: undefined,
      name: undefined,
      clientID: undefined,
      authorizeUrl: undefined,
      redirectUrl: undefined,
      signupUrl: undefined,
      useState: undefined,
      params: undefined
    })()
  })(),
  origins: Record({
    current: Origin(),
    currentPublicKeys: List(),
    currentMembers: List(),
    currentPendingInvitations: List(),
    currentSecrets: List(),
    mine: List(),
    myInvitations: List(),
    currentIntegrations: Record({
      selected: undefined,
      integrations: undefined,
      ui: Record({
        creds: Record({
          validating: false,
          validated: false,
          valid: false,
          message: undefined
        })()
      })()
    })(),
    ui: Record({
      current: Record({
        addingPublicKey: false,
        addingPrivateKey: false,
        creating: false,
        errorMessage: undefined,
        exists: false,
        loading: true,
        publicKeyListErrorMessage: undefined,
        userInviteErrorMessage: undefined,
        integrationsSaveErrorMessage: undefined
      })(),
      mine: Record({
        errorMessage: undefined,
        loading: true,
      })(),
    })(),
  })(),
  packages: Record({
    current: Package(),
    currentChannels: [],
    currentSettings: undefined,
    dashboard: Record({
      origin: undefined,
      recent: List()
    })(),
    latest: Package(),
    latestInChannel: Record({
      stable: undefined,
      unstable: undefined,
      [latestBase]: undefined
    })(),
    visible: List(),
    versions: undefined,
    currentPlatforms: [],
    currentPlatform: undefined,
    nextRange: 0,
    perPage: 50,
    searchQuery: '',
    totalCount: 0,
    ui: Record({
      current: Record({
        creating: false,
        errorMessage: undefined,
        exists: false,
        loading: true,
      })(),
      latest: Record({
        errorMessage: undefined,
        exists: false,
        loading: true,
      })(),
      latestInChannel: Record({
        stable: Record({
          errorMessage: undefined,
          exists: false,
          loading: true,
        })(),
        unstable: Record({
          errorMessage: undefined,
          exists: false,
          loading: true,
        })(),
        [latestBase]: Record({
          errorMessage: undefined,
          exists: false,
          loading: false,
        })()
      })(),
      versions: Record({
        errorMessage: undefined,
        exists: false,
        loading: true,
      })(),
      visible: Record({
        errorMessage: undefined,
        exists: false,
        loading: true,
      })(),
    })(),
  })(),
  projects: Record({
    current: Project(),
    currentProjects: [],
    visible: List(),
    ui: Record({
      current: Record({
        exists: false,
        loading: true,
      })(),
      visible: Record({
        errorMessage: undefined,
        exists: false,
        loading: true,
      })(),
    })()
  })(),
  router: Record({
    requestedRoute: undefined,
    route: Record({
      id: undefined,
      description: undefined,
      params: {},
      url: undefined,
      urlAfterRedirects: undefined
    })()
  })(),
  ui: Record({
    layout: 'default'
  })(),
  users: Record({
    current: Record({
      accessTokens: [],
      newAccessToken: undefined,
      email: undefined,
      failedSignIn: false,
      isSigningIn: false,
      isUserNavOpen: false,
      username: undefined,
      flags: 0,
      profile: Record({
        id: undefined,
        name: undefined,
        email: undefined
      })(),
      license: Record({
        licenseKey: undefined,
        licenseValid: false,
        licenseValidationMessage: undefined,
        validatingLicenseKey: false,
      })(),
      ui: Record({
        accessTokens: Record({
          deleting: false,
          generating: false,
          loading: false
        })()
      })()
    })()
  })(),
  events: Record({
    visible: List(),
    nextRange: 0,
    perPage: 50,
    totalCount: 0,
    searchQuery: '',
    dateFilter: undefined,
    ui: Record({
      visible: Record({
        errorMessage: undefined,
        exists: false,
        loading: true,
      })(),
    })(),
  })(),
  eventsSaas: Record({
    visible: List(),
    nextRange: 0,
    perPage: 50,
    totalCount: 0,
    searchQuery: '',
    dateFilter: undefined,
    ui: Record({
      visible: Record({
        errorMessage: undefined,
        exists: false,
        loading: true,
      })(),
    })(),
  })(),
})();
