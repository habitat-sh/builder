variable "aws_account_id" {
  description = "The AWS account ID. Used by bucket policy"
  default     = "799338195663"
}

variable "env" {
  description = "Name of logical server environment for network"
}

variable "dns_zone_id" {
  description = "DNS Zone for all network"
}

variable "aws_ami" {
  description = "Base AMI for all latest LTS ubuntu nodes"

  default = {
    us-west-2 = "ami-043505d1b57b5d3e3"
  }
}

variable "aws_key_pair" {
  description = "AWS Key Pair name for instances"
}

variable "aws_region" {
  description = "AWS Region"
}

variable "aws_vpc_id" {
  description = "VPC resource id to place security groups into"
}

variable "aws_admin_sg" {
  description = "Administration security group for all instances"
}

variable "hab_sup_sg" {
  description = "AWS security group identifier for Habitat Supervisor gossip connectivity"
}

variable "bldr_url" {
  description = "URL of Builder to receive package updates from"
  default     = "https://bldr.habitat.sh"
}

variable "release_channel" {
  description = "Release channel in Builder to receive package updates from"
  default     = "stable"
}

variable "sup_release_channel" {
  description = "Release channel in Builder to receive Supervisor package updates from"
  default     = "builder-live"
}

variable "worker_release_channel" {
  description = "Release channel in Builder to pull habitat/builder-worker updates from"
  default     = "stable"
}

variable "log_level" {
  description = "Logging level for the Habitat Supervisor"
  default     = "info"
}

variable "gossip_listen_port" {
  description = "Port for Habitat Supervisor's --gossip-listen"
  default     = 9638
}

variable "http_listen_port" {
  description = "Port for Habitat Supervisor's --http-listen"
  default     = 9631
}

variable "public_subnet_id" {
  description = "Identifier for public AWS subnet"
}

variable "private_subnet_id" {
  description = "Identifier for private AWS subnet"
}

variable "worker_studio_subnet_id" {
  description = "Identifier for a Worker's Studio AWS subnet"
}

variable "worker_studio_gateway_ip" {
  description = "IP Address for the Worker's Studio internet gateway"
}

variable "peers" {
  type        = list(string)
  description = "List of addresses for initial Supervisor peer(s)"
}

variable "jobsrv_worker_count" {
  description = "Number of JobSrv workers to start"
}

variable "api_count" {
  description = "Number of frontend/API nodes to start"
}

variable "connection_agent" {
  description = "Set to false to disable using ssh-agent to authenticate"
}

variable "connection_private_key" {
  description = "File path to AWS keypair private key"
}

variable "datadog_api_key" {
  description = "API key for the DataDog agent"
}

variable "instance_size_api" {
  description = "AWS instance size for builder-api server(s)"
}

variable "instance_size_jobsrv" {
  description = "AWS instance size for builder-jobsrv server"
}

variable "instance_size_worker" {
  description = "AWS instance size for builder-worker server(s)"
}

variable "admin_password" {
  description = "Windows Administrator password to login as"
}

variable "windows_worker_count" {
  description = "Number of Windows workers to start"
}

variable "instance_size_windows_worker" {
  description = "AWS instance size for Windows worker server(s)"
}

variable "instance_size_linux2_worker" {
  description = "AWS instance size for Linux2 worker server(s)"
}

variable "linux2_worker_count" {
  description = "Number of Linux2 workers to start"
}

variable "bastion_host" {
  description = "Jump Host to use for SSH connections"
}

variable "bastion_user" {
  description = "Jump Host username for SSH connections"
}

variable "bastion_private_key" {
  description = "File path to the private key to use for SSH connections via the Jump Host"
}

variable "enabled_features" {
  description = "A list of feature names to enable. Use just the feature name, e.g. 'PIDS_FROM_LAUNCHER', rather than the environment variable 'HAB_FEAT_PIDS_FROM_LAUNCHER'."
  type    = list(string)
  default = []
}

variable "aws_public_key" {
  description = "File path to the public key to use for authentication to windows builder"
}
