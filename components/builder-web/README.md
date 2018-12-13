# Builder Web

This is the web application for Builder. It's a single-page application built with [Angular](https://angular.io/), [TypeScript](https://www.typescriptlang.org/), [Redux](http://redux.js.org/) and [Immutable.js](https://facebook.github.io/immutable-js/).

## Development Setup

This section outlines how to get the Builder Web UI running for development on the Host OS, while the backend Builder API service runs inside a Guest VM.

This involves a bit more configuration and steps to get running, however it optimizes and speeds up the UI development workflow.

While it's possible to run this application without a concurrently running Builder API service, you won't be able to perform the kinds of actions that rely on that API (like create an origin, list and browse packages, sign in and out, and so on).

Therefore, these steps are part of the setup of the full dev environment that is outlined in the overall [Builder  Development](../../DEVELOPING.md) doc.

### Prerequisites

You should have gone through the steps in the Builder Development document to set up and configure the Builder API service, and created your Github app.

You will need the Github app id and client id for configuration below.  Your Github app should be pointing to `localhost:3000` for the redirect.

### Repository Setup

Select a location to clone the Builder repo on your Host OS, eg, `~/Workspace` (this directory will be referred to as ${BUILDER_SRC_ROOT} in the sections below)

```
cd ${BUILDER_SRC_ROOT}
git clone https://github.com/habitat-sh/builder.git
```

### Host OS Provisioning

You will need to install `node`.

We suggest using [NVM](https://github.com/creationix/nvm) (Node Version Manager) to install the version of Node specified in [.nvmrc](.nvmrc). Follow [the instructions in the NVM docs](https://github.com/creationix/nvm#installation) to set that up.

Once NVM is installed (you can verify with `nvm --version`), `cd` into `${BUILDER_SRC_ROOT}/components/builder-web` and run:

```
nvm install
```

When that completes, verify the installation:

```
node --version
```

... which should now match what's in `.nvmrc`.

### Configuration

In the `builder-web` directory, copy the `habitat.conf.sample.js` to `habitat.conf.js` to set up your development runtime configuration.

Update the *github_app_id* and *oauth_client_id* fields with the values from your Github app.

### Running the `builder-web` server

To start the node web server on your local machine:

```
npm install
npm start
```

You should now be able to browse to the UI at `http://localhost:3000/#/pkgs`.

Note that browsing to `http://localhost:3000/` (i.e., at the root level) will activate the application's default route, which is configured to redirect signed-out users to the Habitat home page (http://habitat.sh), and various navigation links will operate similarly. If you plan on developing for both the Builder UI and the [Web site](../../www), consider changing some of your configuration entries to allow for easier navigation between the two:

```
...
docs_url: "http://localhost:4567/docs",
tutorials_url: "http://localhost:4567/learn",
www_url: "http://localhost:4567",
...
```

See the [www README](../../www/README.md) for help setting it up.

## Tests

Run the unit tests with `npm test`. They also run in the background when running `npm start`.

Files ending with .test.ts and .spec.ts are unit tested. We use
[Karma](https://karma-runner.github.io/0.13/index.html) and [Jasmine](https://jasmine.github.io/).
See [app/util.test.ts](app/util.test.ts) for an example.

## Tasks

These are defined in [package.json](package.json) and can be run with `npm run
TASK_NAME`.

* `build`: Build the JS and CSS
* `build-css`
* `build-css-watch`: Build the CSS and watch for changes
* `build-js`
* `build-js-watch`
* `clean`: Remove files created by build tasks
* `clean-css`
* `clean-js`
* `lint`: Check TS and SCSS files for lint errors
* `lint-css`
* `lint-css-watch`
* `lint-js`
* `lint-js-watch`
* `repl`: Start a TypeScript REPL
* `start`: Watch for changes and start a development server running on port 3000
* `test`: Run the tests
* `test-watch`
* `test-unit`: Run the unit tests
* `test-unit-watch`
* `travis`: For running the build and tests on Travis CI

## Code Style Conventions

These are guidelines for how to structure and format code in the application.

* Four spaces for tabs.
* TypeScript is linted with [TSLint](http://palantir.github.io/tslint/) using
  additional rules from the [Angular Style Guide](https://angular.io/styleguide).
  The rules followed in this repository are in the [tslint.json](tslint.json) file.
  Check your code with `npm run lint-js`.
* SCSS is linted with [Sass Lint](https://github.com/sasstools/sass-lint). The
  rules followed in this repository are in the [.sass-lint.yml](.sass-lint.yml)
  file. Check your code with `npm run lint-css`.
* TypeScript files should be named the same name as their default export (or the
  main thing they are concerned with, if there is no default export), so if a
  file has `export default class AppComponent {}`, it should be named
  AppComponent.ts. If a module exports many things, it should given an
  appropriate name and use camelCase.
* Directories should be made for components and their associated files when
  there is more than one file that pertains to a component.
* Directories that end in -page/ and components that are SomethingPageComponent
  are "page components", meaning they represent something that functions as a
  page in the app. All of these should be used in the `RouteConfig` of the
  AppComponent.
* Directory names and SCSS file names should use snake-case.
* SCSS files should start with an underscore and use snake-case:
  \_my-thing.scss. (in Sass, files that start with underscore are partials and
  can be loaded into other files. [app/app.scss](app/app.scss) imports these
  files.)

## Tools

* [Visual Studio Code](https://code.visualstudio.com/) works very well with
  TypeScript. There's also a tslint extension.
* The [Redux Devtools Chrome extension](https://chrome.google.com/webstore/detail/redux-devtools/lmhkpmbekcpmknklioeibfkpmmfibljd?hl=en)
  will let you inspect the state and actions of the running app in Chrome.

## Production

This section is primarily a FYI.

The JavaScript and CSS files are built by `npm run build`.

`npm run dist` build these files and puts them along with the index.html and
other needed files into the dist/ directory.

The app is deployed to production with the Builder API Proxy service, with the configuration
in [/terraform](/terraform) and the Habitat plan in
[/components/builder-api-proxy/habitat](/components/builder-api-proxy/habitat).
