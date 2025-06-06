use std::borrow::Cow;

use rspack_cacheable::{cacheable, cacheable_dyn, with::AsPreset};
use rspack_collections::Identifiable;
use rspack_error::{impl_empty_diagnosable_trait, Result};
use rspack_hash::{RspackHash, RspackHashDigest};
use rspack_macros::impl_source_map_config;
use rspack_sources::{BoxSource, RawStringSource, Source, SourceExt};
use rspack_util::source_map::SourceMapKind;

use crate::{
  dependencies_block::AsyncDependenciesBlockIdentifier, impl_module_meta_info, module_update_hash,
  BuildInfo, BuildMeta, CodeGenerationResult, Compilation, ConcatenationScope, Context,
  DependenciesBlock, DependencyId, FactoryMeta, Module, ModuleIdentifier, ModuleType,
  RuntimeGlobals, RuntimeSpec, SourceType,
};

#[impl_source_map_config]
#[cacheable]
#[derive(Debug)]
pub struct RawModule {
  blocks: Vec<AsyncDependenciesBlockIdentifier>,
  dependencies: Vec<DependencyId>,
  #[cacheable(with=AsPreset)]
  source: BoxSource,
  identifier: ModuleIdentifier,
  readable_identifier: String,
  runtime_requirements: RuntimeGlobals,
  factory_meta: Option<FactoryMeta>,
  build_info: BuildInfo,
  build_meta: BuildMeta,
}

static RAW_MODULE_SOURCE_TYPES: &[SourceType] = &[SourceType::JavaScript];

impl RawModule {
  pub fn new(
    source: String,
    identifier: ModuleIdentifier,
    readable_identifier: String,
    runtime_requirements: RuntimeGlobals,
  ) -> Self {
    Self {
      blocks: Default::default(),
      dependencies: Default::default(),
      // TODO: useSourceMap, etc...
      source: RawStringSource::from(source).boxed(),
      identifier,
      readable_identifier,
      runtime_requirements,
      factory_meta: None,
      build_info: BuildInfo {
        cacheable: true,
        strict: true,
        ..Default::default()
      },
      build_meta: Default::default(),
      source_map_kind: SourceMapKind::empty(),
    }
  }
}

impl Identifiable for RawModule {
  fn identifier(&self) -> ModuleIdentifier {
    self.identifier
  }
}

impl DependenciesBlock for RawModule {
  fn add_block_id(&mut self, block: AsyncDependenciesBlockIdentifier) {
    self.blocks.push(block)
  }

  fn get_blocks(&self) -> &[AsyncDependenciesBlockIdentifier] {
    &self.blocks
  }

  fn add_dependency_id(&mut self, dependency: DependencyId) {
    self.dependencies.push(dependency)
  }

  fn remove_dependency_id(&mut self, dependency: DependencyId) {
    self.dependencies.retain(|d| d != &dependency)
  }

  fn get_dependencies(&self) -> &[DependencyId] {
    &self.dependencies
  }
}

#[cacheable_dyn]
#[async_trait::async_trait]
impl Module for RawModule {
  impl_module_meta_info!();

  fn module_type(&self) -> &ModuleType {
    &ModuleType::JsAuto
  }

  fn source_types(&self) -> &[SourceType] {
    RAW_MODULE_SOURCE_TYPES
  }

  fn source(&self) -> Option<&BoxSource> {
    Some(&self.source)
  }

  fn readable_identifier(&self, _context: &Context) -> Cow<str> {
    Cow::Borrowed(&self.readable_identifier)
  }

  fn size(&self, _source_type: Option<&SourceType>, _compilation: Option<&Compilation>) -> f64 {
    f64::max(1.0, self.source.size() as f64)
  }

  // #[tracing::instrument("RawModule::code_generation", skip_all, fields(identifier = ?self.identifier()))]
  async fn code_generation(
    &self,
    _compilation: &crate::Compilation,
    _runtime: Option<&RuntimeSpec>,
    _: Option<ConcatenationScope>,
  ) -> Result<CodeGenerationResult> {
    let mut cgr = CodeGenerationResult::default();
    cgr.runtime_requirements.insert(self.runtime_requirements);
    cgr.add(SourceType::JavaScript, self.source.clone());
    Ok(cgr)
  }

  async fn get_runtime_hash(
    &self,
    compilation: &Compilation,
    runtime: Option<&RuntimeSpec>,
  ) -> Result<RspackHashDigest> {
    let mut hasher = RspackHash::from(&compilation.options.output);
    self.source.dyn_hash(&mut hasher);
    module_update_hash(self, &mut hasher, compilation, runtime);
    Ok(hasher.digest(&compilation.options.output.hash_digest))
  }
}

impl_empty_diagnosable_trait!(RawModule);
