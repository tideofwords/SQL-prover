def parse_query(query):
    pass

class Config:
    def __init__(self, num_cols):
        self.num_cols = num_cols

def generate_circuit(config):
    lines = []
    lines.append("pub struct CircuitInput {")
    lines.append(f"\tpub arr: [Vec<u64>, {config.num_cols}],")
    lines.append("{}")