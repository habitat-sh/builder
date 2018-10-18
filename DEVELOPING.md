# Builder Services Development Environment

## Overview

This document captures the steps to start and run a Builder environment for development. The builder environment includes the builder services, as well as the depot web site.

There are several ways of creating a Builder dev environment - but supporting all operating systems and environments has proven to be untenable. This document includes one officially supported way of creating a Builder dev environment, and links to unsupported ways of creating the dev environment that you may use at your own risk.

## Pre-Requisites

Note that while these instructions should technically work with any linux environment that habitat supports, we recommend either a VMWare-based VM or bare metal. Other providers (e.g., Docker, VirtualBox) have posed difficulty. For instance, VirtualBox doesn't support forwarding privileged ports, which makes using the web app outside the linux environment challenging.

VMWare with Vagrant (and [the supplied Vagrantfile](https://github.com/habitat-sh/builder/blob/master/Vagrantfile)) works well, but Vagrant is not required if you're comfortable with setting up your own VM and port forwarding. For VMWare Fusion 10, adding the following lines to `/Library/Preferences/VMware Fusion/networking` seems to suffice (where 172.16.174.130 is the IP of the VM):
```
add_nat_portfwd 8 tcp 80 172.16.174.130 80
add_nat_portfwd 8 tcp 3000 172.16.174.130 3000
add_nat_portfwd 8 tcp 9636 172.16.174.130 9636
```

### Ports required
1. 9636 - Intra-supervisor communication
1. 80 - Web app
1. 9631 - supervisor api port
1. 5433 - (nonstandard) posgres port (configurable in [datastore.toml](https://github.com/habitat-sh/builder/blob/master/support/builder/datastore.toml#L3))

### Checkout
* If you are developing on Linux
* * Ensure you have curl
* * `git clone https://github.com/habitat-sh/builder.git /src`
* If you are developing on a Mac
* * `git clone https://github.com/habitat-sh/builder.git`

If you are using Linux environment you can run `/src/support/linux/provision.sh` to configure your host
If you are on a Mac, you will need brew, direnv, habitat, and Docker for Mac

### GitHub OAuth Application
`APP_HOSTNAME` mentioned below, will typically be `localhost`

1. [Setup a GitHub application](https://github.com/settings/apps/new) for your GitHub organization
1. Set the value of `Homepage URL` to `http://${APP_HOSTNAME}`
1. Set the value of `User authorization callback URL` to `http://${APP_HOSTNAME}/` (The trailing `/` is *important*)
1. Set the value of `Webhook URL` to `http://${APP_HOSTNAME}/`
1. Set Repository metadata, Repository administration, Repository content and Organization members to read only (this is only used for your org so it's safe)
1. Save and download the private key. It will result in a file like `app-name.date.private-key.pem`
1. Copy the private key to `${HABITAT_SRC_ROOT}/.secrets/builder-github-app.pem`
1. Record the the client-id, client-secret, app_id and public page link (in the left sidebar). These will be used for the `GITHUB_CLIENT_ID`, `GITHUB_CLIENT_SECRET`, `GITHUB_APP_ID` and `GITHUB_APP_URL` build variables (seen below).

### Create app env file

1. `cp ${HABITAT_SRC_ROOT}/.secrets/habitat-env.sample ${HABITAT_SRC_ROOT}/.secrets/habitat-env`
1. Edit `${HABITAT_SRC_ROOT}/.secrets/habitat-env` with the variables from above
1. Save and close

### Studio requirements

Ensure you have run `hab setup` in the environment that will be executing the studio, or exported environment variables of `HAB_ORIGIN` and `HAB_AUTO_TOKEN` whose values correspond with the public Builder service. For example:

```
export HAB_ORIGIN=habitat
export HAB_AUTH_TOKEN=<YOUR_PERSONAL_ACCESS_TOKEN>
```

### Starting the services

From either your VM or Mac:

* `sudo -i # Or your preferred method of running as root`
* `cd <source path>`
* `direnv allow`
* `hab studio enter`
* `start-builder`

### Merging the shards

If you created your development environment before August 17th 2018, you have
a database that contains 128 shards. You need to merge them into one, or your
services will flap and spew errors into the log that say "Shard migration
hasn't been completed successfully".

To migrate your shards, run the following commands from inside your development
studio:


```
PGPASSWORD=$(cat /hab/svc/builder-datastore/config/pwfile) tools/merge-shards/merge-shards.sh jobsrv migrate
PGPASSWORD=$(cat /hab/svc/builder-datastore/config/pwfile) tools/merge-shards/merge-shards.sh sessionsrv migrate
PGPASSWORD=$(cat /hab/svc/builder-datastore/config/pwfile) tools/merge-shards/merge-shards.sh originsrv migrate
```

If all goes well, you'll see some messages about things being all good, and
your services should behave normally. If they don't, exit the studio and
re-enter.

Worst case scenario, just `hab studio rm` and start over. Fresh dev
environments won't have this issue nor will they need to be migrated.

### UI Setup

If you are not doing UI development you just need to navigate to `${APP_HOSTNAME}/#/sign-in`

If you are developing the UI:

* Follow the instructions in the [Web UI README](https://github.com/habitat-sh/builder/blob/master/components/builder-web/README.md) to get the Web UI running locally.
* Open up a browser and navigate to http://localhost:3000/#/pkgs - you should see the Habitat UI running locally.
* In the studio, you will need to run
* * `ui-dev-mode` to swap out the github application for development on `localhost:3000`
* * `upload_github_keys` to update the private key from your app with the new shared key for the app connected to `localhost:3000`
* * Note: Make sure you have copied the private key as described [here](#GitHub OAuth Application)

## Helper functions

* `start-builder` - Run the builder cluster
* `origin <name>` - Create the core origin. Default: core
* `project` - Create a project (you can also configure this in the web UI)
* `build-builder` - Build all the builder components
* `build-<component>` - Ex: `build-router` will build the router component for development and load it
* `dev_docs` - Print the full set of command docs

### Generate a Personal Access Token using the web UI

1. Go to the web UI that you used in the last step
2. Go the Profile page (click on the user icon in the upper right corner to get to it)
3. Click on the 'Generate Token' button
4. Save the token somewhere safe (eg, eg, your .bashrc or Hab cli.toml)

Note: If you need to perform commands where you auth with both the prod site, as well as the local site, you will have to switch the auth tokens appropriately.

### Create a project using the web UI

1. Go the web UI that you used in the last step
2. Go to the origins page, and select your origin
3. Click on the 'Connect a plan file' button
4. Click on 'Install Github App' button to install the Builder Dev app on your github account
5. Go back to the Packages page (from Step 3), and follow the instructions to link the plan you want to build

Note: your GH app must have access to a repo containing a plan file. Forking `habitat-sh/core-plans` is an easy way to test.

## Run a build
`build-builder`

### Install dependencies in your local Builder env

You may use the `load_package` helper to specify a package to upload. Ex:

```
load_package /hab/cache/artifacts/core-*.hart
```

Alternatively, you can use the `on-prem-archive.sh` script from the on-prem repo to do the initial hydration (and sync) of base packages - see the [Synchronizing Packages](#Synchronizing_Packages) section below.

#### Option A: From the Web UI
* Navigate to http://${APP_HOSTNAME}/#/pkgs
* If you are not already logged in, log in.
* Click on "My origins"
* Click on "core"
* Click on the package you wish to build
* Click on "Latest"
* Click on "Build latest version"
* Click on "Build Jobs" and "View the output" to see the job in progress
* The job should complete successfully! Congrats, you have a working build!

#### Option B: From the Command Line

Issue the following command (replace `core/nginx` with your origin and package names):

```
hab bldr job start core/nginx
```

This should create a build job, and then dispatch it to the build worker.

You can view the build progress in the web UI or by viewing `/hab/svc/builder-worker/data/876066265100378112/log_pipe-876066265100378112.log`. Replace `876066265100378112` with the group ID output by the `start` command.

Note: you will need to upload additional packages to the core origin for the `core/nginx` build to succeed. Follow the same procedure as for `core/hab-backline`. Currently `core/gcc` and `core/libedit` are required.

### Receiving metrics

Some services like builder-api and builder-jobsrv send statsd metrics. These are easy to monitor if needed for dev purposes.
The below assumes node and npm is already installed and available.

```
npm install -g statsd-logger
statsd-logger
```

Once statsd-logger is running, it should receive and display any metrics sent by the services.

### Setting up to run Builder builds in development

Initially, your depot will be empty, which means you won't be able to run a successful build. Minimally, you'll need to upload the current, stable version of `core/hab-backline` (which you'll have installed locally as a result of entering the studio). Follow these steps to prepare an empty depot to run successful builds:

  1. Ensure Builder services are running (e.g., via `start-builder`). `hab svc status` should be able to confirm this.

  1. Browse to your local instance of Builder UI (e.g., http://localhost) and sign in.

  1. Navigate to **My Origins &gt; Create Origin** and make a new origin called `core` using the default settings.

  1. Navigate to your Profile and generate a personal access token.

  1. Using that token and the `load_package` helper, upload your locally installed version of `hab-backline`. For example, for Habitat 0.64.1:

      ```
      HAB_AUTH_TOKEN=<YOUR_NEWLY_GENERATED_TOKEN> \
      load_package /hab/cache/artifacts/core-hab-backline-0.64.1-20180928012546-x86_64-linux.hart
      ```

You should now be able to connect a plan file, and run a build, of a simple package (e.g., one with no direct dependencies).

### Synchronizing Packages

You may want to take advantage of the package synchronization capability that is now available via the `on-prem-archive.sh` script that is located in the [on-prem builder repo](https://github.com/habitat-sh/on-prem-builder/blob/master/scripts/on-prem-archive.sh)

Prior to using the script, you will need to ensure that a few tools are in your path - including curl, git, and b2sum. For details, please see the instructions in the [README](https://github.com/habitat-sh/on-prem-builder/blob/master/README.md).
