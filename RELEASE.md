# Releasing Habitat Builder

## Preparing for release

If the release requires downtime, create a status update on statuspage.io . This will post notifications to Slack and Twitter.

## Promote packages

* Log into https://bldr.habitat.sh
* Locate the packages that need promotion
* Click the promote button

## Verify

* Ensure all services are operating normally in sumologic and datadog
* Close out the maintenance notice in statuspage.io as needed

# Releasing habitat/builder-worker after a Habitat release

The `habitat/builder-worker` package has dependencies on the Habitat
build tooling. This determines which version of the Studio and
`core/hab-plan-build` are used to build packages, but also which
version of the Docker exporter is used. As a result, new releases of
the `habitat/builder-worker` package must be built after each Habitat
release.

This is currently automated in the [post_habitat_release
pipeline](.expeditor/post_habitat_release.pipeline.yml). Once packages
are built, they are automatically promoted to the `acceptance`
channel, from which our worker instances in our Acceptance environment
are updating themselves. Here, the new releases can be manually
exercised. If all looks in order, unblock the `post_habitat_release`
pipeline to allow the new packages to be promoted to the `stable`
channel, from which our Production environment workers update
themselves.

*NOTE* The above automation currently only applies to the time after a
new Habitat release. It does _not_ come into play with day-to-day
modifications of `habitat/builder-worker` _itself_, though you are
free to manually promote packages into the `acceptance` channel to try
them out before then promoting them to the `stable` channel.
