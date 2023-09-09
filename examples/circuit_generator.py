def parse_query(query):
    pass

class Config:
    def __init__(self, num_cols):
        self.num_cols = num_cols

def generate_circuit(config):
    lines = []
    lines.append(f"const NUM_COLS: usize = {config.num_cols};")
    lines.append("#[derive(Clone, Debug, Serialize, Deserialize)]")
    lines.append("pub struct CircuitInput {")
    lines.append("\tpub db: [Vec<u64>; NUM_COLS],")
    lines.append("{}")