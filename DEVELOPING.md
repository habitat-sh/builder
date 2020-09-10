# Builder Development

## Overview

This document outlines the steps to configure and run a Builder environment for development. The Builder dev environment includes the Builder API service, as well as the Builder Web UI.

There are potentially multiple ways of creating a Builder dev environment - but supporting various systems and environments has proven to be untenable. This document includes the recommended and supported way of creating a Builder dev environment.

## Prerequisites

* *Linux VM on Mac OS/X Host*. You can use a VMWare Fusion Pro 10 (or later) based VM running on Mac OS/X. Other providers (e.g., Docker, VirtualBox) have posed difficulty. For instance, VirtualBox doesn't support forwarding privileged ports, which makes using the web app outside the Linux environment challenging.

* *Ubuntu Desktop 18.04 LTS*. Other distributions should work, but this is the one we will support.

* *Decision on where you want to run the Web UI*. If you are doing active Web UI development, then you will likely want to run the UI on your Host (Mac OS). If so, there are some extra steps and configuration changes that will be needed, and those are called out below. See the [Web UI README](https://github.com/habitat-sh/builder/blob/master/components/builder-web/README.md) for more info.

## Host OS Provisioning

Your VM will need a static IP assigned for the Builder API to work properly. You can do so by adding the following to **the end** of your `/Library/Preferences/VMware Fusion/vmnet8/dhcpd.conf`:
```
host YOUR_VM_NAME {
   hardware ethernet YOUR_VM_MAC_ADDRESS;
   fixed-address YOUR_VM_IP_ADDRESS;
}
```

See https://one.vg/static-ip-addresses-in-vmware-fusion/ for more detail on getting the correct MAC and IP address values when configuring `dhcpd.conf` for your VM.

Then, you will need to re-start the VMWare networking on your host machine, like so:
```
sudo /Applications/VMware\ Fusion.app/Contents/Library/vmnet-cli --stop
sudo /Applications/VMware\ Fusion.app/Contents/Library/vmnet-cli --start
```

If you plan to run the UI on the Host (Mac) OS, you will need to ensure that the Builder API port (9636) is forwarded from your VM to your host.

You can do this by making a change to the default NAT configuration.

Add the following line under the `[incomingtcp]` section in your `/Library/Preferences/VMware Fusion/vmnet8/nat.conf`:
```
9636 = <VM IP addr>:9636
```

You can use the `ip address` command on your Guest VM to get the IP Address.

Then, you will need to re-start the VMWare networking on your host machine, like so:
```
sudo /Applications/VMware\ Fusion.app/Contents/Library/vmnet-cli --stop
sudo /Applications/VMware\ Fusion.app/Contents/Library/vmnet-cli --start
```

You can test the API port access from your Host OS after starting Builder services (steps below) by issuing the following from the command line:

```
curl -v http://localhost:9636/v1/status
```

This should return a `200 OK`.

For further reference on NAT and port forwarding in Fusion, please refer to the [NAT Configuration](https://docs.vmware.com/en/VMware-Fusion/10.0/com.vmware.fusion.using.doc/GUID-7D8E5A7D-FF0C-4975-A794-FF5A9AE83234.html) page

## Guest OS Provisioning

Before you can successfully build, you need to provision the OS with some basic tools and configuration.

1. Use `visudo` to grant your account the ability to do passwordless sudo. Add a line similar to the following to the end of your sudoers file: `<username> ALL=(ALL) NOPASSWD: ALL`

2. Run the following provisioning script: `./support/linux/provision.sh`
   (Complete the repository setup step below to run this script)

3. Ensure you have your github SSH keys in your `~/.ssh` directory (will need for cloning in the next step)

## Repository Setup

The sections below will walk through the steps for getting the source and configuration ready.

### Builder repo clone
Select a location to clone the Builder repo on your Linux VM, eg, `~/Workspace` (this directory will be referred to as ${BUILDER_SRC_ROOT} in the sections below)

```
cd ${BUILDER_SRC_ROOT}
git clone https://github.com/habitat-sh/builder.git
```

This will clone the Builder repo into your Workspace directory.

### OAuth application setup

You will need to create an OAuth application in GitHub, and use the private key, client id and client secret from the app to configure Builder's environment (below).

`APP_HOSTNAME` mentioned below, will typically be `localhost`.

However, if you are going to be doing Web UI development, and running the Web UI on your Host OS, then you will need to use `localhost:3000` instead of `localhost` for `APP_HOSTNAME`.

1. [Create a new GitHub application](https://github.com/settings/apps/new) in your GitHub account
1. Give it a meaningful `GitHub App name`, e.g., "Builder Local Dev"
1. Set the value of `Homepage URL` to `http://${APP_HOSTNAME}`. A host alias that you define on your workstation pointed to a local IP such as the loopback (127.0.0.1) will suffice for APP_HOSTNAME when testing locally.
1. Set the value of `User authorization callback URL` to `http://${APP_HOSTNAME}/` (The trailing `/` is *important*)
1. Set the value of `Webhook URL` to `http://${APP_HOSTNAME}/` (Optional - only needed for testing builds triggered from github. APP_HOSTNAME will need to be routable on the Internet, `localhost` will not work.)
1. Set Repository metadata, Repository administration, Repository content and Organization members to read only (this is only used for your org so it's safe)
1. Download and save the private key. It will result in a file like `app-name.date.private-key.pem`
1. Record the the client-id, client-secret, app_id and public page link (in the left sidebar). These will be used for the `GITHUB_CLIENT_ID`, `GITHUB_CLIENT_SECRET`, `GITHUB_APP_ID` and `GITHUB_APP_URL` config variables in the section below.

### Builder configuration

1. Copy the GitHub application private key (from section above) to the following location (_Important: name it exactly as shown_) `${BUILDER_SRC_ROOT}/.secrets/builder-github-app.pem`
1. Make a copy of the sample env file: `cp ${BUILDER_SRC_ROOT}/.secrets/habitat-env.sample ${BUILDER_SRC_ROOT}/.secrets/habitat-env`
1. Edit the env file with your favorite editor `${BUILDER_SRC_ROOT}/.secrets/habitat-env` and populate the variables appropriately
1. Save and close the env file

## Builder Services Setup

### Starting Builder services
Once the Builder Repo is configured, Builder services can be started inside the Habitat Studio.

* `cd ${BUILDER_SRC_ROOT}`
* `direnv allow`
* `export HAB_AUTH_TOKEN=your_live_builder_token`
* `ls ~/.hab/cache/keys/habitat-* || hab origin key generate habitat`
* `export HAB_ORIGIN=habitat`
* `hab studio enter`

Once inside the Habitat Studio, you should see a welcome message along with a list of useful commands (Use the `dev_docs` command if you need to print out the commands list again).

You may now start the builder services by issuing the following command: `start-builder`

This will download and run the latest `stable` Builder packages (you will re-build everything locally in a later step).

Run `status` to ensure all the services are up.

You can also run `sl` to output the running Supervisor log as needed.

### Starting the Web UI (Optional)

If you are *NOT* doing UI development and standing up the Web UI on your Host OS, then you don't need to do anything extra. You can just navigate to `${APP_HOSTNAME}/#/sign-in`

If there are recent UI changes not yet promoted to stable that you wish to try out, then follow these additional steps to build and deploy the node/angular assets locally off of your branch:

1. `cd components/builder-api-proxy && build`
1. `source results/last_build.env && hab pkg install results/"${pkg_artifact}"`
1. `stop-builder api-proxy`
1. `start-builder api-proxy`

In the event that you *ARE* developing the UI then you will need to follow the instructions in the [Web UI README](https://github.com/habitat-sh/builder/blob/master/components/builder-web/README.md) to get the Web UI running on your Host OS.


### Personal Access Token generation

Once the Builder services are up, you should generate a Personal Access Token. Currently, this can only be done via the Web UI.

1. Log into the Web UI - eg, navigate to http://${APP_HOSTNAME}/#/sign-in
2. Go the Profile page (click on the user icon in the upper right corner to get to it)
3. Click on the 'Generate Token' button
4. Save the token somewhere for later use (eg, your .bashrc or Hab cli.toml, etc.)

Note: If you need to perform commands where you auth with both the prod site, as well as the local site, remember to switch the auth tokens appropriately.

### Origin creation

You should now be able to create a `core` origin, as well as an origin for yourself.

From within the Habitat Studio, issue the following commands:

* `export HAB_AUTH_TOKEN=<your token>`
* `origin`
* `origin <username>`

This should create the origins appropriately.  Note that the auth token is the Personal Access Token that you generated in the last step.

### Seeding base packages

In order to do package builds locally, at a minimum you will need to seed the your dev repo with the latest version of `core/hab-backline`.

From within your Studio, do the following (for example, using the 0.64.1 version of hab-backline):

```
load_package /hab/cache/artifacts/core-hab-backline-0.64.1-20180928012546-x86_64-linux.hart
```

Alternatively, you can use the `on-prem-archive.sh` script from the on-prem repo to do the initial hydration (and sync) of base packages - see the [Synchronizing Packages](#Synchronizing_Packages) section below.

### Plan file connection

Currently, connecting a plan file is only available from within the Web UI.

1. Go the Builder Web UI
2. Click on _My Origins_, and then select your origin
3. Click on the _Connect a plan file_ button
4. Click on the _Install Github App_ button to install the Builder Dev app on your github account
5. Go back to the Packages page (from Step 3), and follow the instructions to link the plan you want to build

Note: your GitHub app must have access to the repo containing the plan file you are testing. Forking `habitat-sh/core-plans` is an easy way to test, or feel free to create your own repo with a test plan.

### Package build

You can test that the plan file you just connected actually builds by issuing a build command. You can do that either via the Builder Web UI, or via the `hab` cli.

### Option A: From the Web UI
* Navigate to http://${APP_HOSTNAME}/#/pkgs
* If you are not already logged in, log in.
* Click on "My origins"
* Click on your origin
* Click on the package you wish to build
* Click on "Latest"
* Click on "Build latest version"
* Click on "Build Jobs" and "View the output" to see the job in progress
* The job should complete successfully! Congrats, you have a working build!

### Option B: From the Command Line

Issue the following command (replace `origin/package` with your origin and package names):

```
hab bldr job start origin/package
```

This should create a build job, and then dispatch it to the build worker.

You can view the build progress in the web UI or by viewing `/hab/svc/builder-worker/data/876066265100378112/log_pipe-876066265100378112.log`. Replace `876066265100378112` with the group ID output by the `start` command.

Once the build kicks off, you should be able to see the streaming logs for the build job in the Web UI.

## Developing Builder services

Before building Builder you must ensure that your Personal Access Token is set to the production instance of Builder. This can be done by clearing the `HAB_AUTH_TOKEN` environment variable or explicitly setting it to your production token.

`export HAB_AUTH_TOKEN=<your production token>`

If the `HAB_AUTH_TOKEN` is not set correctly, you will likely see an error similar to the following when trying to build.

```
Unloading builder-api
Unloading habitat/builder-api
   : Loading /src/components/builder-api/habitat-dev/plan.sh
   builder-api: Plan loaded
   builder-api: Validating plan metadata
   builder-api: Using HAB_BIN=/hab/pkgs/core/hab/0.79.1/20190410220617/bin/hab for installs, signing, and hashing
   builder-api: hab-plan-build setup
   builder-api: Writing pre_build file
   builder-api: Resolving build dependencies
» Installing core/protobuf-cpp
☁ Determining latest version of core/protobuf-cpp in the 'stable' channel
✗✗✗
✗✗✗ [401 Unauthorized] Please check that you have specified a valid Personal Access Token.
✗✗✗
   builder-api: WARN: Could not find a suitable installed package for 'core/protobuf-cpp'
   builder-api: ERROR: Resolving 'core/protobuf-cpp' failed, should this be built first?
   builder-api: Build time: 0m0s
   builder-api: Exiting on error
ERROR: _build-builder aborted due to error
```

If you are developing the Builder services and changing the back end code, you will want to update the Builder services with the latest code. When first doing this, you will need to issue a full build by doing the following from within your Studio:

`build-builder`

This will build and restart all the services with the changes from your local branch.

Once this is done, you can incrementally change code and re-build only the services that are impacted by specifying the service name, e.g.:

`build-builder api`

## Testing

In order to verify the API functionality, run the automated tests:

`test-builder`

If you'd like to preserve the resultant test data in Postgres, run as follows:

`test-builder preserve`

To view the DEBUG level logs from the API tests:

`test-builder suplogs`

### Testing against pre-release core packages

In some scenarios, it's valuable to test against `core` packages that haven't been promoted to stable yet. Testing these requires some extra effort in the set up, as you will also need to build components from [habitat](https://github.com/habitat-sh/habitat)

#### Build Habitat components

First, you will need to clone https://github.com/habitat-sh/habitat and build a subset of the components. It is important they are built in the correct order so that dependencies are correct at install time. You can use the below snippet to build them, replacing the channel as necessary.
```
git clone https://github.com/habitat-sh/habitat
cd habitat
env HAB_BLDR_CHANNEL=stable HAB_ORIGIN=core hab studio run "for component in hab plan-build backline studio pkg-export-docker; do build components/\$component; done"
```

Next, copy the hart files produced to the `results` directory in your copy of the Builder repository. Assuming your `habitat` and `builder` checkout share the same parent directory:
```
cp habitat/results/core-hab*.hart builder/results/
```

Next, you will need to enter the studio inside the builder directory, install the Habitat harts, and rebuild Builder against them. Once this is complete, you can follow the testing instructions detailed in [the testing readme](test/builder-api/README.md). It is safe to skip the `build-builder` step in that document.  You can also use the `test-builder` helper function, shown below.
```
hab studio enter
hab pkg install results/core-hab*.hart
for component in builder-api builder-api-proxy builder-datastore builder-graph builder-jobsrv builder-minio builder-worker; do
  build components/$component
done

test-builder preserve
```

## Advanced Usage

### Receiving metrics

Some services like builder-api and builder-jobsrv send statsd metrics. These are easy to monitor if needed for dev purposes.

The below assumes node and npm is already installed and available.

```
npm install -g statsd-logger
statsd-logger
```

Once statsd-logger is running, it should receive and display any metrics sent by the services.

### Synchronizing Packages

Follow the instructions for [bootstrapping](https://github.com/habitat-sh/on-prem-builder/blob/master/on-prem-docs/bootstrap-core.md) an on-prem Builder instance.
