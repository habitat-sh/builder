# Airlock

Description

## Motivation

## Installation

The following installation example was performed on an Ubuntu LTS system which matches the production environment of Builder (where Airlock is primarily used) and because user namespaces and `uid`/`gid` mapping are enabled by default so require no extra work to get started. You can quickly replicate this setup using a [Vagrant](https://www.vagrantup.com/) virtual machine running a [Bento box](https://github.com/chef/bento) image with the following `Vagrantfile`:

```ruby
Vagrant.configure("2") do |config|
  config.vm.box = "bento/ubuntu-16.04"

  # Creates a second network interface, eth1 which will be
  # used when calling `vagrant ssh`, leaving eth0 for
  # default/egress networking.
  config.vm.network "private_network", type: "dhcp"
end
```

Once your system is ready, there is one remaining piece of setup (aside from installing software) which consists of placing your current user in the `tty` group. Note that you may need to log out and log back in again after doing this.

```sh
# Add user to `tty` group`
sudo usermod --append --groups tty $(whoami)

# Get Airlock put it on PATH for convenience
# (not strictly required)
sudo hab install core/airlock --binlink --force

# Install Busybox as Airlock requires it and will warn
# if not present
sudo hab install core/busybox-static

# Install the Studio sofware so it's available
sudo hab install core/hab-studio

# Enable artifact cache mounting in airlock for convenience,
# not done in production and is entirely optional
export MOUNT_ARTIFACT_CACHE=true
```

## Usage

### General Purpose

### Habitat Studio

### Using Network Isolation

By default, Airlock performs no network isolation as the setup requires elevated permissions and more setup. However, network isolation is used on the Builder worker nodes to ensure that package builds don't have access to the internal Builder service network.

The management of the Airlock network isolation is done with the `airlock netns` subcommand which has a `create` and `destroy` action. Both of these actions require root access either by running these as the `root` user or with `sudo`.

To start, we're going to assume that there are at least 2 network interfaces present on your host which you can replicate by using the Vagrant machine described in the [Installation](#installation) section. With this setup there are 2 network interfaces:

* `eth0` which can route traffic to the internet and is the default network interface. We are going to give this interface  to Airlock for its networking namespace meaning that while it is in use, the base host will not be able to contact the internet.
* `eth1` which is used when connecting to the Vagrant virtual machine over SSH. We are going to leave this interface alone, because if we give this one over for Airlock networking we lose our connection to the host.

The only remaining task is to determine the gateway address for the `eth0` interface which we can get using `ip route list`. Here's an example:

```sh
> ip route list
default via 192.168.211.2 dev eth0
172.16.129.0/24 dev eth1  proto kernel  scope link  src 172.16.129.133
192.168.211.0/24 dev eth0  proto kernel  scope link  src 192.168.211.143
```

We're going to use the gateway address for `eth0` which is also the default route of `192.168.211.2`. Plugging that into the `airlock netns create` command we get:

```sh
sudo airlock netns create \
  --gateway 192.168.211.2 \
  --interface eth0 \
  --ns-dir /tmp/airlock-ns \
  --user $(whoami)
```

The `$(whoami)` call will print out your current user name, which in the Vagrant virtual machine would be `vagrant`. This is the user which will "own" the created namespaces.

You can enter a minimal shell session in the network namespace with the following (type `exit` to quit):

```sh
airlock run \
  --use-netns /tmp/airlock-ns/netns \
  --use-userns /tmp/airlock-ns/userns \
  sh
```

Note that we're also technically providing a user namespace which is required to support the network namespace. You can check that the IP address for `eth0` is the same and that only `lo0` and `eth0` are present:

```sh
ip addr
ping www.google.com
```

Finally, to destroy the network namespace and return the `eth0` network interface back to the host, call the `airlock netns destroy` command, passing in the namespace directory:

```sh
sudo airlock netns destroy --ns-dir /tmp/airlock-ns
```

## Building

### Local Development

* no external libs required, simple to build
* only on linux systems

```sh
curl https://sh.rustup.rs -sSf | sh
git clone https://github.com/habitat-sh/builder.git
cd builder/components/airlock
cargo build
```

```sh
cargo run -- run sh
```

```sh
../../target/debug/airlock run sh
```

### Build The Habitat Package

* have Habitat setup with website instructions, including origin key setup

```sh
git clone https://github.com/habitat-sh/builder.git
cd builder
hab pkg build components/airlock
```

## Internal Design

## Relationship to Builder Platform

## Further Reading and References

* [Rootless Containers with runC](http://events.linuxfoundation.org/sites/events/files/slides/rootless-containers-2016.pdf) (SUSE engineer presentation with excellent slide of manually creating a "container")
* [Rootless Containers with runC](https://www.cyphar.com/blog/post/20160627-rootless-containers-with-runc) (Blog post with more detail of above presentation)
* [runC Rootless Containers PR](https://github.com/opencontainers/runc/pull/774) (PR based on above presentation to runC project)
* [runC Consoles, Consoles, Consoles PR](https://github.com/opencontainers/runc/pull/1018) (the hard details of getting consoles to work with user namespaces, etc.)
* [remainroot](https://github.com/cyphar/remainroot) (tool that shims out different functions to trick a process into thinking it's able to change its credentials)
* [Build Your Own Docker](http://tailhook.github.io/containers-tutorial/#/step-1) (presentation by author of Vagga with good code examples)
* [Applying Mount Namespaces](https://www.ibm.com/developerworks/library/l-mount-namespaces/index.html) (IBM developerWorks article)
* [Vagga](https://github.com/tailhook/vagga) (a containerization tool without daemons, written in Rust)
* [unshare Crate](https://github.com/tailhook/unshare) (written by author of Vagga)
* [user_namespaces(7)](http://man7.org/linux/man-pages/man7/user_namespaces.7.html) (man page for Linux user namespaces)
* [Hands on Linux sandbox with namespaces and cgroups](https://blogs.rdoproject.org/7761/hands-on-linux-sandbox-with-namespaces-and-cgroups) (great article with mostly working example)
* [Network Namespaces](https://blogs.igalia.com/dpino/2016/04/10/network-namespaces/) (explains how to set up a network namespace with an example)
* [Setup a network namespace with Internet access](https://gist.github.com/dpino/6c0dca1742093346461e11aa8f608a99) (gist with working example)
* [Introducing Linux Network Namespaces](https://blog.scottlowe.org/2013/09/04/introducing-linux-network-namespaces/) (article with example)
* [Understanding and Hardening Linux Containers](https://www.nccgroup.trust/globalassets/our-research/us/whitepapers/2016/april/ncc_group_understanding_hardening_linux_containers-1-1.pdf) (NCC Group Whitepaper which is *really* good and *really* scary)
* [Understanding Container Security](https://docs.cloudfoundry.org/concepts/container-security.html) (Cloud Foundry Documentation. Hardening section has good items to consider, actions to take)
* [What even is a container: namespaces and cgroups](https://jvns.ca/blog/2016/10/10/what-even-is-a-container/) (Good background by Julia Evans)
* [namespaces(7)](http://man7.org/linux/man-pages/man7/namespaces.7.html) (man page for Linux namespaces)
* [Resource management: Linux kernel Namespaces and cgroups](http://www.haifux.org/lectures/299/netLec7.pdf) (presentation for Haifux with lots of good background, although a little dated from 2013)
* [Namespaces in operation, part 5: User namespaces](https://lwn.net/Articles/532593/) (LWN article discussing uid/gid mapping)
* [Namespaces in operation, part 6: more on user namespaces](https://lwn.net/Articles/540087/) (more, I guess?)
* [/proc](http://www.tldp.org/LDP/Linux-Filesystem-Hierarchy/html/proc.html) (Linux From Scratch article explaining the files in the `/proc` virtual filesystem)
* [Mounting the proc and devpts file systems](http://www.iitk.ac.in/LDP/LDP/lfs/5.0/html/chapter06/proc.html) (from Linux From Scratch)
* [Sysdig and CoreOS Meetup Jul '15: Best Practices For Container Environments](https://www.youtube.com/watch?v=gMpldbcMHuI) (First presentation by Brian "Redbeard" from CoreOS is pretty good content)
* [Containers From Scratch](https://ericchiang.github.io/post/containers-from-scratch/) (Overview of the Linux facilities that make "containers" possible)
* [Setting the Record Straight: containers vs. Zones vs. Jails vs. VMs](https://blog.jessfraz.com/post/containers-zones-jails-vms/) (Good writeup by Jessie Frazelle on differences in isolation between Linux and other systems)
* [Getting Towards Real Sandbox Containers](https://blog.jessfraz.com/post/getting-towards-real-sandbox-containers/) (explaining the POC of [binctr](https://github.com/jessfraz/binctr))
* [Linux containers in 500 lines of code](https://blog.lizzie.io/linux-containers-in-500-loc.html) (This article is way more than 500 lines of text long--it's a beast!)
* [Vulnerability Exploitation In Docker Container Environments](https://www.blackhat.com/docs/eu-15/materials/eu-15-Bettini-Vulnerability-Exploitation-In-Docker-Container-Environments.pdf) (Presentation, good history of Linux containment facilities)
* [Unprivileged Build Containers](https://blog.hansenpartnership.com/unprivileged-build-containers/) (Example of creating a container for building software using user namespaces)
* [Introducing Linux Network Namespaces](https://blog.scottlowe.org/2013/09/04/introducing-linux-network-namespaces/) (had a key bit of information: you can assign full network interfaces to a namespace)
* [Join network namespace from inside user namespace](https://stackoverflow.com/questions/42377398/join-network-namespace-from-inside-user-namespace) (explanation why you can't `setns` to a network namespace without also joining the user namespace)
* [runC: The little engine that could (run Docker containers) - Black Belt Track](https://www.youtube.com/watch?v=ZAhzoz2zJj8&feature=youtu.be&t=41m24s) (DockerCon16 presentation with author of above Stack Overflow answer with some more background)

### Man Pages

* [`clone(2)`](https://linux.die.net/man/2/clone)
* [`setns(2)`](https://linux.die.net/man/2/setns)
* [`chown(2)`](https://linux.die.net/man/2/chown)
* [`openpty(3)`](https://linux.die.net/man/3/openpty)
* [`open(2)`](https://linux.die.net/man/2/open)
* [`ptmx(4)`](https://linux.die.net/man/4/ptmx)

## Contributing

See Habitat project docs

## License

See Habitat license