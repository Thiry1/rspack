use rspack_cacheable::{
  cacheable, cacheable_dyn,
  with::{AsOption, AsPreset, AsVec, Skip},
};
use rspack_collections::IdentifierSet;
use rspack_core::{
  create_exports_object_referenced, export_from_import, get_dependency_used_by_exports_condition,
  get_exports_type, property_access, AsContextDependency, ConnectionState, Dependency,
  DependencyCategory, DependencyCodeGeneration, DependencyCondition, DependencyId,
  DependencyLocation, DependencyRange, DependencyTemplate, DependencyTemplateType, DependencyType,
  ExportPresenceMode, ExportsType, ExtendedReferencedExport, FactorizeInfo, ImportAttributes,
  JavascriptParserOptions, ModuleDependency, ModuleGraph, ModuleReferenceOptions, ReferencedExport,
  RuntimeSpec, SharedSourceMap, TemplateContext, TemplateReplaceSource, UsedByExports,
};
use rspack_error::Diagnostic;
use rustc_hash::FxHashSet as HashSet;
use swc_core::ecma::atoms::Atom;

use super::{
  create_resource_identifier_for_esm_dependency,
  esm_import_dependency::esm_import_dependency_get_linking_error, esm_import_dependency_apply,
};

#[cacheable]
#[derive(Debug, Clone)]
pub struct ESMImportSpecifierDependency {
  id: DependencyId,
  #[cacheable(with=AsPreset)]
  request: Atom,
  #[cacheable(with=AsPreset)]
  name: Atom,
  source_order: i32,
  shorthand: bool,
  asi_safe: bool,
  range: DependencyRange,
  #[cacheable(with=AsVec<AsPreset>)]
  ids: Vec<Atom>,
  call: bool,
  direct_import: bool,
  used_by_exports: Option<UsedByExports>,
  #[cacheable(with=AsOption<AsVec<AsPreset>>)]
  referenced_properties_in_destructuring: Option<HashSet<Atom>>,
  resource_identifier: String,
  export_presence_mode: ExportPresenceMode,
  attributes: Option<ImportAttributes>,
  #[cacheable(with=Skip)]
  source_map: Option<SharedSourceMap>,
  pub namespace_object_as_context: bool,
  factorize_info: FactorizeInfo,
}

impl ESMImportSpecifierDependency {
  #[allow(clippy::too_many_arguments)]
  pub fn new(
    request: Atom,
    name: Atom,
    source_order: i32,
    shorthand: bool,
    asi_safe: bool,
    range: DependencyRange,
    ids: Vec<Atom>,
    call: bool,
    direct_import: bool,
    export_presence_mode: ExportPresenceMode,
    referenced_properties_in_destructuring: Option<HashSet<Atom>>,
    attributes: Option<ImportAttributes>,
    source_map: Option<SharedSourceMap>,
  ) -> Self {
    let resource_identifier =
      create_resource_identifier_for_esm_dependency(&request, attributes.as_ref());
    Self {
      id: DependencyId::new(),
      request,
      name,
      source_order,
      shorthand,
      asi_safe,
      range,
      ids,
      call,
      direct_import,
      export_presence_mode,
      used_by_exports: None,
      namespace_object_as_context: false,
      referenced_properties_in_destructuring,
      attributes,
      resource_identifier,
      source_map,
      factorize_info: Default::default(),
    }
  }

  pub fn get_ids<'a>(&'a self, mg: &'a ModuleGraph) -> &'a [Atom] {
    mg.get_dep_meta_if_existing(&self.id)
      .map(|meta| meta.ids.as_slice())
      .unwrap_or_else(|| self.ids.as_slice())
  }

  pub fn get_referenced_exports_in_destructuring(
    &self,
    ids: Option<&[Atom]>,
  ) -> Vec<ExtendedReferencedExport> {
    if let Some(referenced_properties) = &self.referenced_properties_in_destructuring {
      referenced_properties
        .iter()
        .map(|prop| {
          if let Some(v) = ids {
            let mut value = v.to_vec();
            value.push(prop.clone());
            ReferencedExport::new(value, false)
          } else {
            ReferencedExport::new(vec![prop.clone()], false)
          }
        })
        .map(ExtendedReferencedExport::Export)
        .collect::<Vec<_>>()
    } else if let Some(v) = ids {
      vec![ExtendedReferencedExport::Array(v.to_vec())]
    } else {
      create_exports_object_referenced()
    }
  }

  pub fn create_export_presence_mode(options: &JavascriptParserOptions) -> ExportPresenceMode {
    options
      .import_exports_presence
      .or(options.exports_presence)
      .unwrap_or(if let Some(true) = options.strict_export_presence {
        ExportPresenceMode::Error
      } else {
        ExportPresenceMode::Auto
      })
  }
}

#[cacheable_dyn]
impl Dependency for ESMImportSpecifierDependency {
  fn id(&self) -> &DependencyId {
    &self.id
  }

  fn loc(&self) -> Option<DependencyLocation> {
    self.range.to_loc(self.source_map.as_ref())
  }

  fn range(&self) -> Option<&DependencyRange> {
    Some(&self.range)
  }

  fn source_order(&self) -> Option<i32> {
    Some(self.source_order)
  }

  fn get_attributes(&self) -> Option<&ImportAttributes> {
    self.attributes.as_ref()
  }

  fn set_used_by_exports(&mut self, used_by_exports: Option<UsedByExports>) {
    self.used_by_exports = used_by_exports;
  }

  fn category(&self) -> &DependencyCategory {
    &DependencyCategory::Esm
  }

  fn dependency_type(&self) -> &DependencyType {
    &DependencyType::EsmImportSpecifier
  }

  fn get_module_evaluation_side_effects_state(
    &self,
    _module_graph: &ModuleGraph,
    _module_chain: &mut IdentifierSet,
  ) -> ConnectionState {
    ConnectionState::Bool(false)
  }

  fn _get_ids<'a>(&'a self, mg: &'a ModuleGraph) -> &'a [Atom] {
    self.get_ids(mg)
  }

  fn resource_identifier(&self) -> Option<&str> {
    Some(&self.resource_identifier)
  }

  // #[tracing::instrument(skip_all)]
  fn get_diagnostics(&self, module_graph: &ModuleGraph) -> Option<Vec<Diagnostic>> {
    let module = module_graph.get_parent_module(&self.id)?;
    let module = module_graph.module_by_identifier(module)?;
    if let Some(should_error) = self
      .export_presence_mode
      .get_effective_export_presence(&**module)
      && let Some(diagnostic) = esm_import_dependency_get_linking_error(
        self,
        self.get_ids(module_graph),
        module_graph,
        format!("(imported as '{}')", self.name),
        should_error,
      )
    {
      return Some(vec![diagnostic]);
    }
    None
  }

  fn get_referenced_exports(
    &self,
    module_graph: &ModuleGraph,
    _runtime: Option<&RuntimeSpec>,
  ) -> Vec<ExtendedReferencedExport> {
    let mut ids = self.get_ids(module_graph);
    // namespace import
    if ids.is_empty() {
      return self.get_referenced_exports_in_destructuring(None);
    }

    let mut namespace_object_as_context = self.namespace_object_as_context;
    if let Some(id) = ids.first()
      && id == "default"
    {
      let parent_module = module_graph
        .get_parent_module(&self.id)
        .expect("should have parent module");
      let exports_type = get_exports_type(module_graph, &self.id, parent_module);
      match exports_type {
        ExportsType::DefaultOnly | ExportsType::DefaultWithNamed => {
          if ids.len() == 1 {
            return self.get_referenced_exports_in_destructuring(None);
          }
          ids = &ids[1..];
          namespace_object_as_context = true;
        }
        ExportsType::Dynamic => {
          return create_exports_object_referenced();
        }
        _ => {}
      }
    }

    if self.call && !self.direct_import && (namespace_object_as_context || ids.len() > 1) {
      if ids.len() == 1 {
        return create_exports_object_referenced();
      }
      // remove last one
      ids = &ids[..ids.len() - 1];
    }
    self.get_referenced_exports_in_destructuring(Some(ids))
  }

  fn could_affect_referencing_module(&self) -> rspack_core::AffectType {
    rspack_core::AffectType::True
  }
}

#[cacheable_dyn]
impl ModuleDependency for ESMImportSpecifierDependency {
  fn request(&self) -> &str {
    &self.request
  }

  fn user_request(&self) -> &str {
    &self.request
  }

  fn set_request(&mut self, request: String) {
    self.request = request.into();
  }

  fn get_condition(&self) -> Option<DependencyCondition> {
    // TODO: this part depend on inner graph parser plugin to call set_used_by_exports to update the used_by_exports
    get_dependency_used_by_exports_condition(self.id, self.used_by_exports.as_ref())
  }

  fn factorize_info(&self) -> &FactorizeInfo {
    &self.factorize_info
  }

  fn factorize_info_mut(&mut self) -> &mut FactorizeInfo {
    &mut self.factorize_info
  }
}

impl AsContextDependency for ESMImportSpecifierDependency {}

#[cacheable_dyn]
impl DependencyCodeGeneration for ESMImportSpecifierDependency {
  fn dependency_template(&self) -> Option<DependencyTemplateType> {
    Some(ESMImportSpecifierDependencyTemplate::template_type())
  }
}

#[cacheable]
#[derive(Debug, Clone, Default)]
pub struct ESMImportSpecifierDependencyTemplate;

impl ESMImportSpecifierDependencyTemplate {
  pub fn template_type() -> DependencyTemplateType {
    DependencyTemplateType::Dependency(DependencyType::EsmImportSpecifier)
  }
}

impl DependencyTemplate for ESMImportSpecifierDependencyTemplate {
  fn render(
    &self,
    dep: &dyn DependencyCodeGeneration,
    source: &mut TemplateReplaceSource,
    code_generatable_context: &mut TemplateContext,
  ) {
    let dep = dep
      .as_any()
      .downcast_ref::<ESMImportSpecifierDependency>()
      .expect(
        "ESMImportSpecifierDependencyTemplate should only be used for ESMImportSpecifierDependency",
      );
    let TemplateContext {
      compilation,
      runtime,
      concatenation_scope,
      ..
    } = code_generatable_context;
    let module_graph = compilation.get_module_graph();
    // Only available when module factorization is successful.
    let reference_mgm = module_graph.module_graph_module_by_dependency_id(&dep.id);
    let connection = module_graph.connection_by_dependency_id(&dep.id);
    let is_target_active = if let Some(con) = connection {
      con.is_target_active(&module_graph, *runtime)
    } else {
      true
    };

    if !is_target_active {
      return;
    }

    let used = reference_mgm.is_some();
    if reference_mgm.is_some() && !used {
      // TODO do this by PureExpressionDependency.
      let value = format!("/* \"{}\" unused */null", dep.request);
      if dep.shorthand {
        source.insert(dep.range.end, &format!(": {value}"), None);
      } else {
        source.replace(dep.range.start, dep.range.end, &value, None)
      }
      return;
    }

    let ids = dep.get_ids(&module_graph);
    let import_var = compilation.get_import_var(&dep.id);

    let export_expr = if let Some(scope) = concatenation_scope
      && let Some(con) = module_graph.connection_by_dependency_id(&dep.id)
      && scope.is_module_in_scope(con.module_identifier())
    {
      if ids.is_empty() {
        scope.create_module_reference(
          con.module_identifier(),
          &ModuleReferenceOptions {
            asi_safe: Some(dep.asi_safe),
            ..Default::default()
          },
        )
      } else if dep.namespace_object_as_context && ids.len() == 1 {
        // ConcatenationScope::create_module_reference(&dep, module, options)
        scope.create_module_reference(
          con.module_identifier(),
          &ModuleReferenceOptions {
            asi_safe: Some(dep.asi_safe),
            ..Default::default()
          },
        ) + property_access(ids, 0).as_str()
      } else {
        scope.create_module_reference(
          con.module_identifier(),
          &ModuleReferenceOptions {
            asi_safe: Some(dep.asi_safe),
            ids: ids.to_vec(),
            call: dep.call,
            direct_import: dep.direct_import,
            ..Default::default()
          },
        )
      }
    } else {
      esm_import_dependency_apply(dep, dep.source_order, code_generatable_context);
      export_from_import(
        code_generatable_context,
        true,
        &dep.request,
        &import_var,
        ids,
        &dep.id,
        dep.call,
        !dep.direct_import,
        Some(dep.shorthand || dep.asi_safe),
      )
    };

    if dep.shorthand {
      source.insert(dep.range.end, format!(": {export_expr}").as_str(), None);
    } else {
      source.replace(dep.range.start, dep.range.end, export_expr.as_str(), None);
    }
  }
}
