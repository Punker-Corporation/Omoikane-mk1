use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PassId(usize);

impl PassId {
    pub const fn index(self) -> usize {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ResourceId(String);

impl ResourceId {
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for ResourceId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for ResourceId {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl fmt::Display for ResourceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

fn insert_initial_resource(
    live_resources: &mut BTreeMap<ResourceId, usize>,
    resource_lifetimes: &mut Vec<ResourceLifetime>,
    resource: ResourceId,
    kind: ResourceKind,
) {
    let lifetime_index = resource_lifetimes.len();
    live_resources.insert(resource.clone(), lifetime_index);
    resource_lifetimes.push(ResourceLifetime {
        resource,
        kind,
        created_by: None,
        discarded_by: None,
        last_used_by: None,
    });
}

fn insert_pass_created_resource(
    live_resources: &mut BTreeMap<ResourceId, usize>,
    resource_lifetimes: &mut Vec<ResourceLifetime>,
    resource: ResourceId,
    kind: ResourceKind,
    pass: PassId,
) {
    let lifetime_index = resource_lifetimes.len();
    live_resources.insert(resource.clone(), lifetime_index);
    resource_lifetimes.push(ResourceLifetime {
        resource,
        kind,
        created_by: Some(pass),
        discarded_by: None,
        last_used_by: None,
    });
}

fn mark_resource_used(
    live_resources: &BTreeMap<ResourceId, usize>,
    resource_lifetimes: &mut [ResourceLifetime],
    resource: &ResourceId,
    pass: PassId,
) {
    if let Some(lifetime_index) = live_resources.get(resource) {
        resource_lifetimes[*lifetime_index].last_used_by = Some(pass);
    }
}

fn push_unavailable_resource(
    errors: &mut Vec<RenderGraphError>,
    unavailable_resources: &mut BTreeSet<ResourceId>,
    pass: PassId,
    resource: &ResourceId,
) {
    if unavailable_resources.insert(resource.clone()) {
        errors.push(RenderGraphError::ResourceReadBeforeWrite {
            pass,
            resource: resource.clone(),
        });
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderPass {
    name: String,
    depends_on: BTreeSet<PassId>,
    reads: BTreeSet<ResourceId>,
    writes: BTreeSet<ResourceId>,
    transient_creates: BTreeSet<ResourceId>,
    persistent_creates: BTreeSet<ResourceId>,
    preserves: BTreeSet<ResourceId>,
    discards: BTreeSet<ResourceId>,
}

impl RenderPass {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            depends_on: BTreeSet::new(),
            reads: BTreeSet::new(),
            writes: BTreeSet::new(),
            transient_creates: BTreeSet::new(),
            persistent_creates: BTreeSet::new(),
            preserves: BTreeSet::new(),
            discards: BTreeSet::new(),
        }
    }

    pub fn after(mut self, pass: PassId) -> Self {
        self.depends_on.insert(pass);
        self
    }

    pub fn read(mut self, resource: impl Into<ResourceId>) -> Self {
        self.reads.insert(resource.into());
        self
    }

    pub fn write(mut self, resource: impl Into<ResourceId>) -> Self {
        self.writes.insert(resource.into());
        self
    }

    pub fn create_transient(mut self, resource: impl Into<ResourceId>) -> Self {
        self.transient_creates.insert(resource.into());
        self
    }

    pub fn create_persistent(mut self, resource: impl Into<ResourceId>) -> Self {
        self.persistent_creates.insert(resource.into());
        self
    }

    pub fn preserve(mut self, resource: impl Into<ResourceId>) -> Self {
        self.preserves.insert(resource.into());
        self
    }

    pub fn discard(mut self, resource: impl Into<ResourceId>) -> Self {
        self.discards.insert(resource.into());
        self
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn depends_on(&self) -> &BTreeSet<PassId> {
        &self.depends_on
    }

    pub fn reads(&self) -> &BTreeSet<ResourceId> {
        &self.reads
    }

    pub fn writes(&self) -> &BTreeSet<ResourceId> {
        &self.writes
    }

    pub fn creates(&self) -> &BTreeSet<ResourceId> {
        &self.transient_creates
    }

    pub fn transient_creates(&self) -> &BTreeSet<ResourceId> {
        &self.transient_creates
    }

    pub fn persistent_creates(&self) -> &BTreeSet<ResourceId> {
        &self.persistent_creates
    }

    pub fn preserves(&self) -> &BTreeSet<ResourceId> {
        &self.preserves
    }

    pub fn discards(&self) -> &BTreeSet<ResourceId> {
        &self.discards
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RenderGraph {
    imported_resources: BTreeSet<ResourceId>,
    persistent_resources: BTreeSet<ResourceId>,
    passes: Vec<RenderPass>,
}

impl RenderGraph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn import_resource(&mut self, resource: impl Into<ResourceId>) -> &mut Self {
        self.imported_resources.insert(resource.into());
        self
    }

    pub fn declare_persistent_resource(&mut self, resource: impl Into<ResourceId>) -> &mut Self {
        self.persistent_resources.insert(resource.into());
        self
    }

    pub fn add_pass(&mut self, pass: RenderPass) -> PassId {
        let id = PassId(self.passes.len());
        self.passes.push(pass);
        id
    }

    pub fn add_dependency(
        &mut self,
        pass: PassId,
        depends_on: PassId,
    ) -> Result<&mut Self, RenderGraphError> {
        if pass.index() >= self.passes.len() {
            return Err(RenderGraphError::UnknownPassDependency { pass, depends_on });
        }
        if depends_on.index() >= self.passes.len() {
            return Err(RenderGraphError::UnknownPassDependency { pass, depends_on });
        }
        self.passes[pass.index()].depends_on.insert(depends_on);
        Ok(self)
    }

    pub fn passes(&self) -> &[RenderPass] {
        &self.passes
    }

    pub fn validate(&self) -> Result<GraphValidation, Vec<RenderGraphError>> {
        let mut errors = Vec::new();
        let mut live_resources = BTreeMap::<ResourceId, usize>::new();
        let mut resource_lifetimes = Vec::<ResourceLifetime>::new();
        let mut read_since_write = BTreeMap::<ResourceId, bool>::new();
        let execution_order = validate_pass_dependencies(&self.passes, &mut errors);

        for resource in &self.imported_resources {
            insert_initial_resource(
                &mut live_resources,
                &mut resource_lifetimes,
                resource.clone(),
                ResourceKind::Imported,
            );
        }
        for resource in &self.persistent_resources {
            if live_resources.contains_key(resource) {
                errors.push(RenderGraphError::ResourceDeclaredTwice {
                    resource: resource.clone(),
                });
                continue;
            }
            insert_initial_resource(
                &mut live_resources,
                &mut resource_lifetimes,
                resource.clone(),
                ResourceKind::Persistent,
            );
        }

        for pass_id in &execution_order {
            let pass = &self.passes[pass_id.index()];
            validate_pass_shape(pass_id, pass, &mut errors);
            let mut unavailable_resources = BTreeSet::new();

            for resource in pass
                .transient_creates
                .iter()
                .chain(pass.persistent_creates.iter())
            {
                if live_resources.contains_key(resource) {
                    errors.push(RenderGraphError::ResourceCreatedTwice {
                        pass: *pass_id,
                        resource: resource.clone(),
                    });
                }
            }

            for resource in &pass.transient_creates {
                if !live_resources.contains_key(resource) {
                    insert_pass_created_resource(
                        &mut live_resources,
                        &mut resource_lifetimes,
                        resource.clone(),
                        ResourceKind::Transient,
                        *pass_id,
                    );
                }
            }
            for resource in &pass.persistent_creates {
                if !live_resources.contains_key(resource) {
                    insert_pass_created_resource(
                        &mut live_resources,
                        &mut resource_lifetimes,
                        resource.clone(),
                        ResourceKind::Persistent,
                        *pass_id,
                    );
                }
            }

            for resource in &pass.reads {
                let created_here = pass.transient_creates.contains(resource)
                    || pass.persistent_creates.contains(resource);
                if !created_here && !live_resources.contains_key(resource) {
                    push_unavailable_resource(
                        &mut errors,
                        &mut unavailable_resources,
                        *pass_id,
                        resource,
                    );
                }
                mark_resource_used(&live_resources, &mut resource_lifetimes, resource, *pass_id);
                read_since_write.insert(resource.clone(), true);
            }

            for resource in &pass.preserves {
                let created_here = pass.persistent_creates.contains(resource);
                if !created_here && !live_resources.contains_key(resource) {
                    push_unavailable_resource(
                        &mut errors,
                        &mut unavailable_resources,
                        *pass_id,
                        resource,
                    );
                }
                mark_resource_used(&live_resources, &mut resource_lifetimes, resource, *pass_id);
                read_since_write.insert(resource.clone(), true);
            }

            for resource in &pass.writes {
                let created_here = pass.transient_creates.contains(resource)
                    || pass.persistent_creates.contains(resource);
                if !created_here && !live_resources.contains_key(resource) {
                    push_unavailable_resource(
                        &mut errors,
                        &mut unavailable_resources,
                        *pass_id,
                        resource,
                    );
                    continue;
                }

                let wrote_before = read_since_write.contains_key(resource);
                let was_read = read_since_write.get(resource).copied().unwrap_or(false);
                if wrote_before && !was_read && !created_here {
                    errors.push(RenderGraphError::DuplicateWrite {
                        pass: *pass_id,
                        resource: resource.clone(),
                    });
                }
                mark_resource_used(&live_resources, &mut resource_lifetimes, resource, *pass_id);
                read_since_write.insert(resource.clone(), false);
            }

            for resource in &pass.discards {
                if let Some(lifetime_index) = live_resources.remove(resource) {
                    let lifetime = &mut resource_lifetimes[lifetime_index];
                    lifetime.discarded_by = Some(*pass_id);
                    lifetime.last_used_by = Some(*pass_id);
                } else {
                    errors.push(RenderGraphError::DiscardedUnknownResource {
                        pass: *pass_id,
                        resource: resource.clone(),
                    });
                }
                read_since_write.remove(resource);
            }
        }

        if errors.is_empty() {
            Ok(GraphValidation {
                execution_order,
                resource_lifetimes,
            })
        } else {
            Err(errors)
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphValidation {
    execution_order: Vec<PassId>,
    resource_lifetimes: Vec<ResourceLifetime>,
}

impl GraphValidation {
    pub fn execution_order(&self) -> &[PassId] {
        &self.execution_order
    }

    pub fn resource_lifetimes(&self) -> &[ResourceLifetime] {
        &self.resource_lifetimes
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceLifetime {
    resource: ResourceId,
    kind: ResourceKind,
    created_by: Option<PassId>,
    discarded_by: Option<PassId>,
    last_used_by: Option<PassId>,
}

impl ResourceLifetime {
    pub fn resource(&self) -> &ResourceId {
        &self.resource
    }

    pub const fn kind(&self) -> ResourceKind {
        self.kind
    }

    pub const fn created_by(&self) -> Option<PassId> {
        self.created_by
    }

    pub const fn discarded_by(&self) -> Option<PassId> {
        self.discarded_by
    }

    pub const fn last_used_by(&self) -> Option<PassId> {
        self.last_used_by
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceKind {
    Imported,
    Transient,
    Persistent,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenderGraphError {
    EmptyPass {
        pass: PassId,
    },
    ResourceDeclaredTwice {
        resource: ResourceId,
    },
    ResourceCreatedTwice {
        pass: PassId,
        resource: ResourceId,
    },
    ResourceReadBeforeWrite {
        pass: PassId,
        resource: ResourceId,
    },
    DuplicateWrite {
        pass: PassId,
        resource: ResourceId,
    },
    DiscardedUnknownResource {
        pass: PassId,
        resource: ResourceId,
    },
    ConflictingResourceUse {
        pass: PassId,
        resource: ResourceId,
        uses: Vec<ResourceUse>,
    },
    UnknownPassDependency {
        pass: PassId,
        depends_on: PassId,
    },
    PassDependencyCycle {
        cycle: Vec<PassId>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ResourceUse {
    Read,
    Write,
    CreateTransient,
    CreatePersistent,
    Preserve,
    Discard,
}

fn validate_pass_dependencies(
    passes: &[RenderPass],
    errors: &mut Vec<RenderGraphError>,
) -> Vec<PassId> {
    for (index, pass) in passes.iter().enumerate() {
        let pass_id = PassId(index);
        for dependency in &pass.depends_on {
            if dependency.index() >= passes.len() {
                errors.push(RenderGraphError::UnknownPassDependency {
                    pass: pass_id,
                    depends_on: *dependency,
                });
            }
        }
    }

    let valid_dependencies = passes
        .iter()
        .enumerate()
        .map(|(index, pass)| {
            (
                PassId(index),
                pass.depends_on
                    .iter()
                    .filter(|dependency| dependency.index() < passes.len())
                    .copied()
                    .collect::<BTreeSet<_>>(),
            )
        })
        .collect::<BTreeMap<_, _>>();

    let mut ordered = Vec::with_capacity(passes.len());
    let mut temporary = BTreeSet::new();
    let mut permanent = BTreeSet::new();
    let mut stack = Vec::new();

    for index in 0..passes.len() {
        visit_pass_dependency(
            PassId(index),
            &valid_dependencies,
            &mut temporary,
            &mut permanent,
            &mut stack,
            &mut ordered,
            errors,
        );
    }

    ordered
}

fn visit_pass_dependency(
    pass: PassId,
    dependencies: &BTreeMap<PassId, BTreeSet<PassId>>,
    temporary: &mut BTreeSet<PassId>,
    permanent: &mut BTreeSet<PassId>,
    stack: &mut Vec<PassId>,
    ordered: &mut Vec<PassId>,
    errors: &mut Vec<RenderGraphError>,
) {
    if permanent.contains(&pass) {
        return;
    }
    if temporary.contains(&pass) {
        if let Some(start) = stack.iter().position(|item| *item == pass) {
            let mut cycle = stack[start..].to_vec();
            cycle.push(pass);
            errors.push(RenderGraphError::PassDependencyCycle { cycle });
        }
        return;
    }

    temporary.insert(pass);
    stack.push(pass);

    if let Some(pass_dependencies) = dependencies.get(&pass) {
        for dependency in pass_dependencies {
            visit_pass_dependency(
                *dependency,
                dependencies,
                temporary,
                permanent,
                stack,
                ordered,
                errors,
            );
        }
    }

    stack.pop();
    temporary.remove(&pass);
    permanent.insert(pass);
    ordered.push(pass);
}

fn validate_pass_shape(pass_id: &PassId, pass: &RenderPass, errors: &mut Vec<RenderGraphError>) {
    if pass.writes.is_empty()
        && pass.transient_creates.is_empty()
        && pass.persistent_creates.is_empty()
        && pass.preserves.is_empty()
        && pass.discards.is_empty()
    {
        errors.push(RenderGraphError::EmptyPass { pass: *pass_id });
    }

    for (resource, uses) in resource_uses(pass) {
        let allowed = matches!(
            uses.as_slice(),
            [ResourceUse::Read]
                | [ResourceUse::Write]
                | [ResourceUse::CreateTransient]
                | [ResourceUse::CreatePersistent]
                | [ResourceUse::Preserve]
                | [ResourceUse::Discard]
                | [ResourceUse::Read, ResourceUse::Write]
                | [ResourceUse::Write, ResourceUse::CreateTransient]
                | [ResourceUse::Write, ResourceUse::CreatePersistent]
                | [ResourceUse::Write, ResourceUse::Preserve]
                | [ResourceUse::Read, ResourceUse::Write, ResourceUse::Preserve]
                | [
                    ResourceUse::Write,
                    ResourceUse::CreatePersistent,
                    ResourceUse::Preserve
                ]
                | [ResourceUse::Read, ResourceUse::Preserve]
                | [ResourceUse::Read, ResourceUse::Discard]
        );
        if !allowed {
            errors.push(RenderGraphError::ConflictingResourceUse {
                pass: *pass_id,
                resource,
                uses,
            });
        }
    }
}

fn resource_uses(pass: &RenderPass) -> BTreeMap<ResourceId, Vec<ResourceUse>> {
    let mut uses = BTreeMap::<ResourceId, BTreeSet<ResourceUse>>::new();
    insert_uses(&mut uses, &pass.reads, ResourceUse::Read);
    insert_uses(&mut uses, &pass.writes, ResourceUse::Write);
    insert_uses(
        &mut uses,
        &pass.transient_creates,
        ResourceUse::CreateTransient,
    );
    insert_uses(
        &mut uses,
        &pass.persistent_creates,
        ResourceUse::CreatePersistent,
    );
    insert_uses(&mut uses, &pass.preserves, ResourceUse::Preserve);
    insert_uses(&mut uses, &pass.discards, ResourceUse::Discard);
    uses.into_iter()
        .map(|(resource, uses)| (resource, uses.into_iter().collect()))
        .collect()
}

fn insert_uses(
    uses: &mut BTreeMap<ResourceId, BTreeSet<ResourceUse>>,
    resources: &BTreeSet<ResourceId>,
    resource_use: ResourceUse,
) {
    for resource in resources {
        uses.entry(resource.clone())
            .or_default()
            .insert(resource_use);
    }
}

#[cfg(test)]
mod tests {
    use super::{
        PassId, RenderGraph, RenderGraphError, RenderPass, ResourceId, ResourceKind, ResourceUse,
    };

    #[test]
    fn graph_accepts_linear_create_read_write_discard_flow() {
        let mut graph = RenderGraph::new();
        graph.add_pass(
            RenderPass::new("gbuffer")
                .create_transient("color")
                .write("color"),
        );
        graph.add_pass(
            RenderPass::new("postprocess")
                .read("color")
                .write("color")
                .preserve("color"),
        );
        graph.add_pass(RenderPass::new("cleanup").discard("color"));

        let validation = graph.validate().unwrap();
        assert_eq!(
            validation.execution_order(),
            &[PassId(0), PassId(1), PassId(2)]
        );
    }

    #[test]
    fn graph_allows_imported_frame_targets() {
        let mut graph = RenderGraph::new();
        graph.import_resource("swapchain");
        graph.add_pass(
            RenderPass::new("present")
                .write("swapchain")
                .preserve("swapchain"),
        );

        assert!(graph.validate().is_ok());
    }

    #[test]
    fn graph_allows_declared_persistent_resources() {
        let mut graph = RenderGraph::new();
        graph.declare_persistent_resource("history");
        graph.add_pass(RenderPass::new("temporal").read("history").write("history"));

        assert!(graph.validate().is_ok());
    }

    #[test]
    fn graph_orders_explicit_pass_dependencies() {
        let mut graph = RenderGraph::new();
        graph.import_resource("swapchain");
        let upload = graph.add_pass(RenderPass::new("upload").preserve("swapchain"));
        let draw = graph.add_pass(RenderPass::new("draw").after(upload).write("swapchain"));
        graph
            .add_dependency(draw, upload)
            .expect("dependency uses known passes");

        let validation = graph.validate().unwrap();
        assert_eq!(validation.execution_order(), &[upload, draw]);
    }

    #[test]
    fn graph_rejects_unknown_pass_dependencies() {
        let mut graph = RenderGraph::new();
        graph.import_resource("swapchain");
        graph.add_pass(RenderPass::new("draw").after(PassId(9)).write("swapchain"));

        assert_eq!(
            graph.validate().unwrap_err(),
            vec![RenderGraphError::UnknownPassDependency {
                pass: PassId(0),
                depends_on: PassId(9),
            }]
        );
    }

    #[test]
    fn graph_rejects_pass_dependency_cycles() {
        let mut graph = RenderGraph::new();
        graph.import_resource("swapchain");
        let first = graph.add_pass(RenderPass::new("first").write("swapchain"));
        let second = graph.add_pass(RenderPass::new("second").after(first).preserve("swapchain"));
        graph.add_dependency(first, second).unwrap();

        assert_eq!(
            graph.validate().unwrap_err(),
            vec![RenderGraphError::PassDependencyCycle {
                cycle: vec![first, second, first],
            }]
        );
    }

    #[test]
    fn graph_rejects_read_before_write() {
        let mut graph = RenderGraph::new();
        graph.add_pass(RenderPass::new("lighting").read("gbuffer").write("lit"));

        assert_eq!(
            graph.validate().unwrap_err(),
            vec![
                RenderGraphError::ResourceReadBeforeWrite {
                    pass: PassId(0),
                    resource: ResourceId::from("gbuffer"),
                },
                RenderGraphError::ResourceReadBeforeWrite {
                    pass: PassId(0),
                    resource: ResourceId::from("lit"),
                },
            ]
        );
    }

    #[test]
    fn graph_rejects_duplicate_write_without_intervening_read() {
        let mut graph = RenderGraph::new();
        graph.add_pass(
            RenderPass::new("first")
                .create_transient("color")
                .write("color"),
        );
        graph.add_pass(RenderPass::new("second").write("color"));

        assert_eq!(
            graph.validate().unwrap_err(),
            vec![RenderGraphError::DuplicateWrite {
                pass: PassId(1),
                resource: ResourceId::from("color"),
            }]
        );
    }

    #[test]
    fn graph_rejects_resource_created_twice_while_live() {
        let mut graph = RenderGraph::new();
        graph.add_pass(
            RenderPass::new("first")
                .create_transient("color")
                .write("color"),
        );
        graph.add_pass(
            RenderPass::new("second")
                .create_transient("color")
                .write("color"),
        );

        assert_eq!(
            graph.validate().unwrap_err(),
            vec![RenderGraphError::ResourceCreatedTwice {
                pass: PassId(1),
                resource: ResourceId::from("color"),
            }]
        );
    }

    #[test]
    fn graph_allows_discarded_resource_to_be_recreated() {
        let mut graph = RenderGraph::new();
        graph.add_pass(
            RenderPass::new("first")
                .create_transient("color")
                .write("color"),
        );
        graph.add_pass(RenderPass::new("drop").discard("color"));
        graph.add_pass(
            RenderPass::new("second")
                .create_transient("color")
                .write("color"),
        );

        assert!(graph.validate().is_ok());
    }

    #[test]
    fn graph_validation_reports_resource_lifetimes() {
        let mut graph = RenderGraph::new();
        graph.import_resource("swapchain");
        graph.declare_persistent_resource("history");
        graph.add_pass(
            RenderPass::new("gbuffer")
                .create_transient("color")
                .write("color"),
        );
        graph.add_pass(
            RenderPass::new("present")
                .read("color")
                .write("swapchain")
                .preserve("history")
                .discard("color"),
        );

        let validation = graph.validate().unwrap();
        let lifetimes = validation.resource_lifetimes();
        assert_eq!(lifetimes.len(), 3);
        assert_eq!(lifetimes[0].resource(), &ResourceId::from("swapchain"));
        assert_eq!(lifetimes[0].kind(), ResourceKind::Imported);
        assert_eq!(lifetimes[0].last_used_by(), Some(PassId(1)));
        assert_eq!(lifetimes[1].resource(), &ResourceId::from("history"));
        assert_eq!(lifetimes[1].kind(), ResourceKind::Persistent);
        assert_eq!(lifetimes[1].last_used_by(), Some(PassId(1)));
        assert_eq!(lifetimes[2].resource(), &ResourceId::from("color"));
        assert_eq!(lifetimes[2].kind(), ResourceKind::Transient);
        assert_eq!(lifetimes[2].created_by(), Some(PassId(0)));
        assert_eq!(lifetimes[2].discarded_by(), Some(PassId(1)));
        assert_eq!(lifetimes[2].last_used_by(), Some(PassId(1)));
    }

    #[test]
    fn graph_allows_preserve_as_frame_boundary_effect() {
        let mut graph = RenderGraph::new();
        graph.add_pass(
            RenderPass::new("history")
                .create_persistent("history")
                .write("history")
                .preserve("history"),
        );

        assert!(graph.validate().is_ok());
    }

    #[test]
    fn graph_rejects_empty_and_conflicting_passes() {
        let mut graph = RenderGraph::new();
        graph.add_pass(RenderPass::new("empty").read("color"));
        graph.add_pass(
            RenderPass::new("bad")
                .create_transient("depth")
                .read("depth")
                .discard("depth"),
        );

        assert_eq!(
            graph.validate().unwrap_err(),
            vec![
                RenderGraphError::EmptyPass { pass: PassId(0) },
                RenderGraphError::ResourceReadBeforeWrite {
                    pass: PassId(0),
                    resource: ResourceId::from("color"),
                },
                RenderGraphError::ConflictingResourceUse {
                    pass: PassId(1),
                    resource: ResourceId::from("depth"),
                    uses: vec![
                        ResourceUse::Read,
                        ResourceUse::CreateTransient,
                        ResourceUse::Discard,
                    ],
                },
            ]
        );
    }

    #[test]
    fn graph_rejects_dead_read_only_pass() {
        let mut graph = RenderGraph::new();
        graph.import_resource("color");
        graph.add_pass(RenderPass::new("dead").read("color"));

        assert_eq!(
            graph.validate().unwrap_err(),
            vec![RenderGraphError::EmptyPass { pass: PassId(0) }]
        );
    }

    #[test]
    fn graph_rejects_invalid_transient_alias_shape() {
        let mut graph = RenderGraph::new();
        graph.add_pass(
            RenderPass::new("bad")
                .create_transient("scratch")
                .create_persistent("scratch")
                .write("scratch"),
        );

        assert_eq!(
            graph.validate().unwrap_err(),
            vec![RenderGraphError::ConflictingResourceUse {
                pass: PassId(0),
                resource: ResourceId::from("scratch"),
                uses: vec![
                    ResourceUse::Write,
                    ResourceUse::CreateTransient,
                    ResourceUse::CreatePersistent,
                ],
            }]
        );
    }

    #[test]
    fn graph_rejects_use_after_discard() {
        let mut graph = RenderGraph::new();
        graph.add_pass(
            RenderPass::new("make")
                .create_transient("color")
                .write("color"),
        );
        graph.add_pass(RenderPass::new("drop").discard("color"));
        graph.add_pass(RenderPass::new("late").read("color").write("color"));

        assert_eq!(
            graph.validate().unwrap_err(),
            vec![RenderGraphError::ResourceReadBeforeWrite {
                pass: PassId(2),
                resource: ResourceId::from("color"),
            },]
        );
    }

    #[test]
    fn graph_rejects_resource_declared_as_imported_and_persistent() {
        let mut graph = RenderGraph::new();
        graph.import_resource("history");
        graph.declare_persistent_resource("history");
        graph.add_pass(RenderPass::new("history").preserve("history"));

        assert_eq!(
            graph.validate().unwrap_err(),
            vec![RenderGraphError::ResourceDeclaredTwice {
                resource: ResourceId::from("history"),
            }]
        );
    }
}
