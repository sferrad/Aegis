terraform {
  required_providers {
    kubernetes = {
      source  = "hashicorp/kubernetes"
      version = "~> 2.30"
    }
  }
}

provider "kubernetes" {
  config_path = "~/.kube/config"
}

resource "kubernetes_namespace" "dev" {
  metadata {
    name = "dev"
  }
}

resource "kubernetes_deployment" "aegis" {
  #checkov:skip=CKV_K8S_43:Image importee localement dans k3d pour P3 ; le pin par digest sera applique en P4 avec le registry GHCR
  #checkov:skip=CKV_K8S_15:imagePullPolicy Always inadapte a une image locale k3d ; sera gere en P4 avec le pull depuis GHCR
  metadata {
    name      = "aegis"
    namespace = kubernetes_namespace.dev.metadata[0].name
  }

  spec {
    replicas = 1

    selector {
      match_labels = {
        app = "aegis"
      }
    }

    template {
      metadata {
        labels = {
          app = "aegis"
        }
      }

      spec {
        security_context {
          run_as_non_root = true
          run_as_user     = 65532
        }

        container {
          name  = "aegis"
          image = "aegis:0.1.0"

          port {
            container_port = 8080
          }
          liveness_probe {
            http_get {
              path = "/health"
              port = 8080
            }
            initial_delay_seconds = 5
            period_seconds        = 10
          }

          readiness_probe {
            http_get {
              path = "/health"
              port = 8080
            }
            initial_delay_seconds = 5
            period_seconds        = 10
          }
          security_context {
            read_only_root_filesystem  = true
            allow_privilege_escalation = false

            capabilities {
              drop = ["ALL"]
            }
          }

          resources {
            limits = {
              cpu    = "100m"
              memory = "64Mi"
            }
            requests = {
              cpu    = "50m"
              memory = "32Mi"
            }
          }
        }
      }
    }
  }
}