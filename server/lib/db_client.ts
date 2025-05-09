import { createClient } from "@supabase/supabase-js";
import { SUPABASE_URL, SUPABASE_API_KEY } from "../env/server-env";

const supabaseUrl = SUPABASE_URL;
const supabaseKey = SUPABASE_API_KEY;
const supabase = createClient(supabaseUrl, supabaseKey);

export default supabase;
