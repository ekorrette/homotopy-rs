use crate::{diagram::NewDiagramError, rewrite::Label, Diagram, DiagramN, Generator};

pub trait GeneratorInfo {
    fn diagram(&self) -> &Diagram;
    fn is_invertible(&self) -> bool;
}

pub trait Signature {
    type Info: GeneratorInfo;
    fn generators(&self) -> Vec<Generator>;
    fn generator_info(&self, g: Generator) -> Option<&Self::Info>;

    fn label_equiv(&self, x: Label, y: Label) -> bool {
        x == y
    }
}

/// Helper struct for building signatures in tests and benchmarks.
#[derive(Clone, Debug, Default)]
pub struct SignatureBuilder(Vec<GeneratorData>);

#[derive(Clone, Debug)]
pub struct GeneratorData(Generator, Diagram);

impl SignatureBuilder {
    pub fn add_zero(&mut self) -> Diagram {
        let generator = Generator::new(self.0.len(), 0);
        self.0.push(GeneratorData(generator, generator.into()));
        generator.into()
    }

    pub fn add(
        &mut self,
        source: impl Into<Diagram>,
        target: impl Into<Diagram>,
    ) -> Result<DiagramN, NewDiagramError> {
        let source: Diagram = source.into();
        let target: Diagram = target.into();
        let generator = Generator::new(self.0.len(), source.dimension() + 1);
        let diagram = DiagramN::from_generator(generator, source, target)?;
        self.0
            .push(GeneratorData(generator, diagram.clone().into()));
        Ok(diagram)
    }
}

impl GeneratorInfo for GeneratorData {
    fn diagram(&self) -> &Diagram {
        &self.1
    }

    fn is_invertible(&self) -> bool {
        self.0.dimension > 0
    }
}

impl Signature for SignatureBuilder {
    type Info = GeneratorData;

    fn generators(&self) -> Vec<Generator> {
        self.0.iter().map(|gd| gd.0).collect()
    }

    fn generator_info(&self, g: Generator) -> Option<&GeneratorData> {
        self.0.get(g.id)
    }
}
