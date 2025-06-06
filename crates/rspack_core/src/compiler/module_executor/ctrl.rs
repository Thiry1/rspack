use std::collections::VecDeque;

use rspack_collections::IdentifierMap;
use rustc_hash::{FxHashMap as HashMap, FxHashSet as HashSet};
use tokio::sync::mpsc::{error::TryRecvError, UnboundedReceiver};

use super::{
  entry::EntryTask,
  execute::{BeforeExecuteBuildTask, ExecuteTask},
};
use crate::{
  compiler::make::repair::MakeTaskContext,
  utils::task_loop::{Task, TaskResult, TaskType},
  Dependency, DependencyId, LoaderImportDependency, ModuleIdentifier,
};

#[derive(Debug)]
struct UnfinishCounter {
  is_building: bool,
  unfinished_child_module_count: usize,
}

impl UnfinishCounter {
  fn new() -> Self {
    UnfinishCounter {
      is_building: true,
      unfinished_child_module_count: 0,
    }
  }

  fn set_unfinished_child_module_count(&mut self, count: usize) {
    self.is_building = false;
    self.unfinished_child_module_count = count;
  }

  fn minus_one(&mut self) {
    if self.is_building || self.unfinished_child_module_count == 0 {
      panic!("UnfinishDepCount Error")
    }
    self.unfinished_child_module_count -= 1;
  }

  fn is_finished(&self) -> bool {
    !self.is_building && self.unfinished_child_module_count == 0
  }
}

#[derive(Debug, Default)]
struct ExecuteTaskList(Vec<Box<dyn Task<MakeTaskContext>>>);

impl ExecuteTaskList {
  fn add_task(&mut self, task: ExecuteTask) {
    self.0.push(Box::new(task));
    if self.0.len() > 10000 {
      // TODO change to Err
      panic!("ExecuteTaskList exceeds limit and may contain circular build dependencies.")
    }
  }

  fn into_vec(self) -> Vec<Box<dyn Task<MakeTaskContext>>> {
    self.0
  }
}

#[derive(Debug)]
pub enum ExecuteParam {
  DependencyId(DependencyId),
  Entry(Box<LoaderImportDependency>, Option<String>),
}

// send event can only use in sync task
#[derive(Debug)]
pub enum Event {
  StartBuild(ModuleIdentifier),
  // origin_module_identifier and current dependency id and target_module_identifier
  FinishDeps(
    Option<ModuleIdentifier>,
    DependencyId,
    Option<ModuleIdentifier>,
  ),
  // current_module_identifier and sub dependency count
  FinishModule(ModuleIdentifier, usize),
  ExecuteModule(ExecuteParam, ExecuteTask, Option<BeforeExecuteBuildTask>),
  Stop(),
}

#[derive(Debug)]
pub struct CtrlTask {
  pub event_receiver: UnboundedReceiver<Event>,
  execute_task_map: HashMap<DependencyId, ExecuteTaskList>,
  running_module_map: IdentifierMap<UnfinishCounter>,
}

impl CtrlTask {
  pub fn new(event_receiver: UnboundedReceiver<Event>) -> Self {
    Self {
      event_receiver,
      execute_task_map: Default::default(),
      running_module_map: Default::default(),
    }
  }
}

#[async_trait::async_trait]
impl Task<MakeTaskContext> for CtrlTask {
  fn get_task_type(&self) -> TaskType {
    TaskType::Async
  }

  async fn background_run(mut self: Box<Self>) -> TaskResult<MakeTaskContext> {
    while let Some(event) = self.event_receiver.recv().await {
      tracing::info!("CtrlTask async receive {:?}", event);
      match event {
        Event::StartBuild(module_identifier) => {
          self
            .running_module_map
            .insert(module_identifier, UnfinishCounter::new());
        }
        Event::FinishDeps(origin_module_identifier, dep_id, target_module_graph) => {
          if let Some(target_module_graph) = target_module_graph {
            if self.running_module_map.contains_key(&target_module_graph)
              && Some(target_module_graph) != origin_module_identifier
            {
              continue;
            }
          }

          // target module finished
          let Some(origin_module_identifier) = origin_module_identifier else {
            // origin_module_identifier is none means entry dep
            let mut tasks = self
              .execute_task_map
              .remove(&dep_id)
              .expect("should have execute task")
              .into_vec();
            tasks.push(self);
            return Ok(tasks);
          };

          let value = self
            .running_module_map
            .get_mut(&origin_module_identifier)
            .expect("should have counter");
          value.minus_one();
          if value.is_finished() {
            return Ok(vec![Box::new(FinishModuleTask {
              ctrl_task: self,
              module_identifier: origin_module_identifier,
            })]);
          }
        }
        Event::FinishModule(mid, size) => {
          let value = self
            .running_module_map
            .get_mut(&mid)
            .expect("should have counter");
          value.set_unfinished_child_module_count(size);
          if value.is_finished() {
            return Ok(vec![Box::new(FinishModuleTask {
              ctrl_task: self,
              module_identifier: mid,
            })]);
          }
        }
        Event::ExecuteModule(param, execute_task, before_execute_task) => {
          match param {
            ExecuteParam::Entry(dep, layer) => {
              let dep_id = dep.id();
              if let Some(tasks) = self.execute_task_map.get_mut(dep_id) {
                tasks.add_task(execute_task)
              } else {
                let mut list = ExecuteTaskList::default();
                list.add_task(execute_task);
                self.execute_task_map.insert(*dep_id, list);
                return Ok(if let Some(before_execute_task) = before_execute_task {
                  vec![
                    Box::new(before_execute_task),
                    Box::new(EntryTask { dep, layer }),
                    self,
                  ]
                } else {
                  vec![Box::new(EntryTask { dep, layer }), self]
                });
              }
            }
            ExecuteParam::DependencyId(dep_id) => {
              if let Some(tasks) = self.execute_task_map.get_mut(&dep_id) {
                tasks.add_task(execute_task)
              } else {
                return Ok(if let Some(before_execute_task) = before_execute_task {
                  vec![Box::new(before_execute_task), Box::new(execute_task), self]
                } else {
                  vec![Box::new(execute_task), self]
                });
              }
            }
          };
        }
        Event::Stop() => {
          return Ok(vec![]);
        }
      }
    }
    // if channel has been closed, finish this task
    Ok(vec![])
  }
}

#[derive(Debug)]
struct FinishModuleTask {
  ctrl_task: Box<CtrlTask>,
  module_identifier: ModuleIdentifier,
}
#[async_trait::async_trait]
impl Task<MakeTaskContext> for FinishModuleTask {
  fn get_task_type(&self) -> TaskType {
    TaskType::Sync
  }

  async fn main_run(self: Box<Self>, context: &mut MakeTaskContext) -> TaskResult<MakeTaskContext> {
    let Self {
      mut ctrl_task,
      module_identifier,
    } = *self;
    let mut res: Vec<Box<dyn Task<MakeTaskContext>>> = vec![];
    let module_graph =
      MakeTaskContext::get_module_graph_mut(&mut context.artifact.module_graph_partial);
    let mut queue = VecDeque::new();
    queue.push_back(module_identifier);

    // clean ctrl task events
    loop {
      let event = ctrl_task.event_receiver.try_recv();
      tracing::info!("CtrlTask sync receive {:?}", event);
      let Ok(event) = event else {
        if matches!(event, Err(TryRecvError::Empty)) {
          break;
        } else {
          panic!("clean ctrl_task event failed");
        }
      };

      match event {
        Event::StartBuild(module_identifier) => {
          ctrl_task
            .running_module_map
            .insert(module_identifier, UnfinishCounter::new());
        }
        Event::FinishDeps(origin_module_identifier, dep_id, target_module_graph) => {
          if let Some(target_module_graph) = target_module_graph {
            if ctrl_task
              .running_module_map
              .contains_key(&target_module_graph)
              && Some(target_module_graph) != origin_module_identifier
            {
              continue;
            }
          }

          // target module finished
          let Some(origin_module_identifier) = origin_module_identifier else {
            // origin_module_identifier is none means entry dep
            let execute_task = ctrl_task
              .execute_task_map
              .remove(&dep_id)
              .expect("should have execute task")
              .into_vec();
            res.extend(execute_task);
            continue;
          };

          let value = ctrl_task
            .running_module_map
            .get_mut(&origin_module_identifier)
            .expect("should have counter");
          value.minus_one();
          if value.is_finished() {
            queue.push_back(origin_module_identifier);
          }
        }
        Event::FinishModule(mid, size) => {
          let value = ctrl_task
            .running_module_map
            .get_mut(&mid)
            .expect("should have counter");
          value.set_unfinished_child_module_count(size);
          if value.is_finished() {
            queue.push_back(mid);
          }
        }
        Event::ExecuteModule(param, execute_task, before_execute_build_task) => {
          match param {
            ExecuteParam::Entry(dep, layer) => {
              let dep_id = dep.id();
              if let Some(tasks) = ctrl_task.execute_task_map.get_mut(dep_id) {
                tasks.add_task(execute_task)
              } else {
                let mut list = ExecuteTaskList::default();
                list.add_task(execute_task);
                ctrl_task.execute_task_map.insert(*dep_id, list);
                if let Some(before_execute_build_task) = before_execute_build_task {
                  res.push(Box::new(before_execute_build_task));
                }
                res.push(Box::new(EntryTask { dep, layer }));
              }
            }
            ExecuteParam::DependencyId(dep_id) => {
              if let Some(tasks) = ctrl_task.execute_task_map.get_mut(&dep_id) {
                tasks.add_task(execute_task)
              } else {
                if let Some(before_execute_build_task) = before_execute_build_task {
                  res.push(Box::new(before_execute_build_task));
                }
                res.push(Box::new(execute_task));
              }
            }
          };
        }
        Event::Stop() => {
          return Ok(vec![]);
        }
      }
    }

    while let Some(module_identifier) = queue.pop_front() {
      tracing::info!("finish build module {:?}", module_identifier);
      ctrl_task.running_module_map.remove(&module_identifier);

      let mgm = module_graph
        .module_graph_module_by_identifier(&module_identifier)
        .expect("should have mgm");

      let mut original_module_identifiers = HashSet::default();
      for dep_id in mgm.incoming_connections() {
        let connection = module_graph
          .connection_by_dependency_id(dep_id)
          .expect("should have connection");
        if let Some(original_module_identifier) = &connection.original_module_identifier {
          // skip self reference
          if original_module_identifier != &module_identifier {
            original_module_identifiers.insert(*original_module_identifier);
          }
        } else {
          // entry
          let execute_task = ctrl_task
            .execute_task_map
            .remove(&connection.dependency_id)
            .expect("should have execute task")
            .into_vec();
          res.extend(execute_task);
        }
      }

      for id in original_module_identifiers {
        let value = ctrl_task
          .running_module_map
          .get_mut(&id)
          .expect("should have counter");
        value.minus_one();
        if value.is_finished() {
          queue.push_back(id);
        }
      }
    }

    res.push(ctrl_task);
    Ok(res)
  }
}
