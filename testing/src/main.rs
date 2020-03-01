mod config;
mod crash_test;
mod docker_api;
mod fs_util;
mod script;
#[cfg(target_family = "unix")]
use crate::docker_api::{pull_image, LINUX_IMAGE};
#[cfg(target_family = "windows")]
use crate::docker_api::{pull_image, LINUX_IMAGE, MS_IMAGE};
use config::{parse_config, AssignmentsMap};
use grpc_api::test_server::{Test, TestServer};
use grpc_api::{
    AssignmentIdRequest, AssignmentIdResponse, AssignmentMsg, AssignmentResult, VecAssignmentsShort,
};
use structopt::StructOpt;
use tonic::{transport::Server, Request, Response, Status};
use uuid::Uuid;

#[derive(Debug)]
pub struct Tester {
    assignments: AssignmentsMap,
    docker: bollard::Docker,
}

impl Tester {
    fn new(assignments: AssignmentsMap, docker: bollard::Docker) -> Self {
        Tester {
            assignments,
            docker,
        }
    }
}

#[tonic::async_trait]
impl Test for Tester {
    async fn run_test(
        &self,
        request: Request<AssignmentMsg>,
    ) -> Result<Response<AssignmentResult>, Status> {
        let msg = request.into_inner();
        // Eror handling when no valid uuid
        let id = Uuid::parse_str(&msg.assignment_id).unwrap();
        if let Some(assignment) = self.assignments.get(&id) {
            let reply = match crash_test::run(assignment, &msg.source_code, &self.docker).await {
                Err(crash_test::Error::CantCreatTempFile(e)) | Err(crash_test::Error::Copy(e)) => {
                    //wait_print_err(e).await;
                    panic!("{:?}", e);
                }
                Err(crash_test::Error::Docker(e)) => panic!(e),
                Err(e) => AssignmentResult {
                    passed: false,
                    message: Some(e.to_string()),
                    mark: None,
                },
                Ok(_) => AssignmentResult {
                    passed: true,
                    message: None,
                    mark: None,
                },
            };
            Ok(Response::new(reply))
        } else {
            Err(tonic::Status::new(
                tonic::Code::InvalidArgument,
                "assignmentId was not found",
            ))
        }
    }

    async fn get_assignments(
        &self,
        _: Request<()>,
    ) -> Result<Response<VecAssignmentsShort>, Status> {
        let reply = assignments_to_msg(self.assignments.clone());
        Ok(Response::new(reply))
    }
    async fn assignment_exists(
        &self,
        request: Request<AssignmentIdRequest>,
    ) -> Result<Response<AssignmentIdResponse>, Status> {
        //TODO fix unwrap
        let id = Uuid::parse_str(&request.into_inner().assignment_id).unwrap();
        let ret = self.assignments.get(&id).map(|x| x.clone()).is_some();

        Ok(Response::new(AssignmentIdResponse { found: ret }))
    }
}

// impl From<grpc_api::Assignment> for config::Assignment {
//     fn from(assignment: grpc_api::Assignment) -> Self {
//         config::Assignment {
//             name: assignment.name.clone(),
//             solution_path: Path::new(&assignment.solution).to_path_buf(),
//             include_files: assignment
//                 .include_files
//                 .iter()
//                 .map(|p| Path::new(&p).to_path_buf())
//                 .collect::<Vec<_>>(),
//             script_type: assignment.script_type.into(),
//             args: assignment.args.clone(),
//         }
//     }
// }

fn assignments_to_msg(thing: AssignmentsMap) -> VecAssignmentsShort {
    let a = thing
        .into_iter()
        .map(|(id, a)| grpc_api::AssignmentShort {
            name: a.name,
            assignment_id: id.to_string(),
        })
        .collect::<_>();
    VecAssignmentsShort { assignments: a }
}

fn default_port() -> u16 {
    50051
}

#[derive(serde::Deserialize, Debug)]
pub struct ServerConfig {
    #[serde(default = "default_port")]
    port: u16,
}

#[derive(Debug, StructOpt)]
struct Opt {
    #[structopt(short, long, help = "File for all assignments")]
    config: std::path::PathBuf,
    #[structopt(short, long, help = "Convert windows newlines into unix newlines")]
    dos_to_unix: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    std::env::set_var("RUST_LOG", "info");
    env_logger::init();
    let opt = Opt::from_args();
    let config = parse_config(&opt.config)?;
    if opt.dos_to_unix {
        //convert_dos_to_unix(&config).await?;
        log::info!("Done converting")
    }
    log::info!("Exercise: {}", &config.name);
    let docker =
        bollard::Docker::connect_with_local_defaults().expect("Can't connect to docker api.");
    log::info!("Pulling image. This takes some time...");

    #[cfg(target_family = "windows")]
    {
        pull_image(LINUX_IMAGE, &docker).await;
        pull_image(MS_IMAGE, &docker).await;
    }
    #[cfg(target_family = "unix")]
    {
        pull_image(LINUX_IMAGE, &docker).await;
    }

    log::info!("Pulling image done.");
    let test = Tester::new(config.assignments, docker);
    let port = envy::from_env::<ServerConfig>()?.port;
    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], port));
    log::info!("Tester listening on {}", &addr);
    Server::builder()
        .add_service(TestServer::new(test))
        .serve(addr)
        .await?;
    Ok(())
}
