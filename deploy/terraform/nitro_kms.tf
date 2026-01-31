# CHINJU Protocol - Nitro Enclaves KMS Configuration
#
# This Terraform module creates a KMS key with attestation-based
# access control for Nitro Enclaves.
#
# Usage:
#   1. Update the PCR values with your Enclave's actual values
#   2. terraform init
#   3. terraform plan
#   4. terraform apply

terraform {
  required_providers {
    aws = {
      source  = "hashicorp/aws"
      version = "~> 5.0"
    }
  }
}

# Variables
variable "aws_region" {
  description = "AWS region"
  type        = string
  default     = "ap-northeast-1"
}

variable "environment" {
  description = "Environment name (dev, staging, prod)"
  type        = string
  default     = "dev"
}

variable "enclave_pcr0" {
  description = "Expected PCR0 value (Enclave image hash)"
  type        = string
  default     = "" # Set this to your Enclave's PCR0
}

variable "enclave_pcr1" {
  description = "Expected PCR1 value (Kernel hash)"
  type        = string
  default     = "" # Set this to your Enclave's PCR1
}

variable "enclave_pcr2" {
  description = "Expected PCR2 value (Application hash)"
  type        = string
  default     = "" # Set this to your Enclave's PCR2
}

variable "enable_attestation" {
  description = "Enable attestation-based access control (disable for dev)"
  type        = bool
  default     = true
}

# Data sources
data "aws_caller_identity" "current" {}

data "aws_iam_session_context" "current" {
  arn = data.aws_caller_identity.current.arn
}

# IAM Role for Enclave host EC2 instance
resource "aws_iam_role" "enclave_host" {
  name = "chinju-enclave-host-${var.environment}"

  assume_role_policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Action = "sts:AssumeRole"
        Effect = "Allow"
        Principal = {
          Service = "ec2.amazonaws.com"
        }
      }
    ]
  })

  tags = {
    Project     = "chinju-protocol"
    Environment = var.environment
    Component   = "nitro-enclave"
  }
}

# IAM Instance Profile
resource "aws_iam_instance_profile" "enclave_host" {
  name = "chinju-enclave-host-${var.environment}"
  role = aws_iam_role.enclave_host.name
}

# KMS Key for Enclave data encryption
resource "aws_kms_key" "enclave" {
  description             = "CHINJU Protocol Enclave Key (${var.environment})"
  deletion_window_in_days = 30
  enable_key_rotation     = true
  multi_region            = false

  # Key policy with attestation-based access control
  policy = jsonencode({
    Version = "2012-10-17"
    Id      = "chinju-enclave-key-policy"
    Statement = concat([
      # Allow root account full access
      {
        Sid    = "AllowRootAccess"
        Effect = "Allow"
        Principal = {
          AWS = "arn:aws:iam::${data.aws_caller_identity.current.account_id}:root"
        }
        Action   = "kms:*"
        Resource = "*"
      },
      # Allow key administrators
      {
        Sid    = "AllowAdminAccess"
        Effect = "Allow"
        Principal = {
          AWS = data.aws_iam_session_context.current.issuer_arn
        }
        Action = [
          "kms:Create*",
          "kms:Describe*",
          "kms:Enable*",
          "kms:List*",
          "kms:Put*",
          "kms:Update*",
          "kms:Revoke*",
          "kms:Disable*",
          "kms:Get*",
          "kms:Delete*",
          "kms:TagResource",
          "kms:UntagResource",
          "kms:ScheduleKeyDeletion",
          "kms:CancelKeyDeletion"
        ]
        Resource = "*"
      }
    ],
    # Enclave access with attestation (conditional)
    var.enable_attestation && var.enclave_pcr0 != "" ? [
      {
        Sid    = "AllowEnclaveDecrypt"
        Effect = "Allow"
        Principal = {
          AWS = aws_iam_role.enclave_host.arn
        }
        Action = [
          "kms:Decrypt",
          "kms:GenerateDataKey",
          "kms:GenerateDataKeyWithoutPlaintext"
        ]
        Resource = "*"
        Condition = {
          StringEquals = merge(
            var.enclave_pcr0 != "" ? {
              "kms:RecipientAttestation:PCR0" = var.enclave_pcr0
            } : {},
            var.enclave_pcr1 != "" ? {
              "kms:RecipientAttestation:PCR1" = var.enclave_pcr1
            } : {},
            var.enclave_pcr2 != "" ? {
              "kms:RecipientAttestation:PCR2" = var.enclave_pcr2
            } : {}
          )
        }
      }
    ] : [
      # Development mode: no attestation required
      {
        Sid    = "AllowEnclaveDecryptDev"
        Effect = "Allow"
        Principal = {
          AWS = aws_iam_role.enclave_host.arn
        }
        Action = [
          "kms:Decrypt",
          "kms:GenerateDataKey",
          "kms:GenerateDataKeyWithoutPlaintext",
          "kms:Encrypt"
        ]
        Resource = "*"
      }
    ])
  })

  tags = {
    Project     = "chinju-protocol"
    Environment = var.environment
    Component   = "nitro-enclave"
  }
}

# KMS Key Alias
resource "aws_kms_alias" "enclave" {
  name          = "alias/chinju-enclave-${var.environment}"
  target_key_id = aws_kms_key.enclave.key_id
}

# IAM Policy for Enclave host to use KMS
resource "aws_iam_role_policy" "enclave_kms" {
  name = "chinju-enclave-kms-${var.environment}"
  role = aws_iam_role.enclave_host.id

  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Effect = "Allow"
        Action = [
          "kms:Decrypt",
          "kms:GenerateDataKey",
          "kms:GenerateDataKeyWithoutPlaintext"
        ]
        Resource = aws_kms_key.enclave.arn
      },
      {
        Effect = "Allow"
        Action = [
          "kms:DescribeKey"
        ]
        Resource = aws_kms_key.enclave.arn
      }
    ]
  })
}

# Outputs
output "kms_key_id" {
  description = "KMS Key ID"
  value       = aws_kms_key.enclave.key_id
}

output "kms_key_arn" {
  description = "KMS Key ARN"
  value       = aws_kms_key.enclave.arn
}

output "kms_key_alias" {
  description = "KMS Key Alias"
  value       = aws_kms_alias.enclave.name
}

output "enclave_role_arn" {
  description = "IAM Role ARN for Enclave host"
  value       = aws_iam_role.enclave_host.arn
}

output "instance_profile_name" {
  description = "Instance Profile name for EC2"
  value       = aws_iam_instance_profile.enclave_host.name
}

output "environment_variables" {
  description = "Environment variables for Enclave configuration"
  value = {
    AWS_KMS_KEY_ID           = aws_kms_key.enclave.arn
    AWS_REGION               = var.aws_region
    CHINJU_ENCLAVE_PCR0      = var.enclave_pcr0
    CHINJU_ENCLAVE_PCR1      = var.enclave_pcr1
    CHINJU_ENCLAVE_PCR2      = var.enclave_pcr2
    CHINJU_ENCLAVE_ALLOW_DEBUG = !var.enable_attestation
  }
}
