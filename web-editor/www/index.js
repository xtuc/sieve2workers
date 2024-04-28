import * as wasm from "sieve2workers";
import * as prettierPluginEstree from "prettier/plugins/estree";
import * as prettier from "prettier/standalone";
import parserBabel from "prettier/plugins/babel";

import React from "react";
import { createRoot } from "react-dom/client";

import Editor from "@monaco-editor/react";

const demoSieve = `
require ["variables", "relational", "body", "spamtestplus", "fileinto"];

### Sieve generated for save-on-SMTP identities {{{
# You do not have any identities with special filing.
### }}}

### Sieve generated for blocked senders {{{
# You have no blocked senders.
### }}}

### Sieve generated for disabled masked email addresses {{{
if header :contains "Fastmail-MaskedEmail" " state=disabled" {
  # addflag "\\Seen";
  # fileinto :specialuse "\\Trash" "INBOX.Trash";
  stop;
}
### }}}

### Sieve generated for spam protection {{{
if not header :matches "X-Spam-Known-Sender" "yes*" {
  if allof(
      header :contains "X-Backscatter" "yes",
      not header :matches "X-LinkName" "*"
  ) {
    set "spam" "Y";
  }
  # if header :value "ge" :comparator "i;ascii-numeric" "X-Spam-score" "5" {
  #   set "spam" "Y";
  # }
}
### }}}

### MailFetch Implicit Keep {{{
### }}}

### Address rules {{{
# You have no address rules
### }}}

### Execute spam filing {{{
if string :is "\${spam}" "Y" {
  # fileinto "\\Junk";
  stop;
}
### }}}

if body :contains "MAKE MONEY FAST" {
    discard;
}

if body :contains "to be saved" {
    fileinto "r2://BINDING_NAME";
}

if spamtest :value "lt" :comparator "i;ascii-numeric" "37" {
    discard;
}
`;

async function compile(input) {
  try {
    const js = wasm.compile(input);
    const jsPretty = await prettier.format(js, {
      semi: false,
      parser: "babel",
      plugins: [parserBabel, prettierPluginEstree],
    });

    return jsPretty;
  } catch (err) {
    console.error(err);
    throw err;
  }
}

class App extends React.Component {
  constructor(props) {
    super(props);
    this.state = {
      jsCode: "",
      error: null,
    };

    this.onChange = this.onChange.bind(this);
  }

  componentDidMount() {
    this.onChange(demoSieve);
  }

  async onChange(newValue) {
    try {
      const out = await compile(newValue);
      this.setState({ jsCode: out, error: null });
    } catch (error) {
      this.setState({ error });
    }
  }

  render() {
    const options = {
      minimap: {
        enabled: false,
      },

      wordWrap: "on",
    };

    return (
      <>
        <div className="code-editor code-editor-sieve">
          <div className="code-editor-header">
            <span className="control"></span>
            <span className="control"></span>
            <span className="control"></span>

            <span className="text">Code Editor: Sieve Input</span>
          </div>

          <Editor
            height="94vh"
            theme="vs-dark"
            options={options}
            defaultLanguage="sieve"
            defaultValue={demoSieve}
            onChange={this.onChange}
          />
        </div>

        <div className="code-editor code-editor-js">
          <div className="code-editor-header">
            <span className="control"></span>
            <span className="control"></span>
            <span className="control"></span>

            <span className="text">Code Editor: Cloudflare Worker Output</span>
          </div>

          {this.state.error ? (
            <div className="error">
              <h1>Failed to compile</h1>
              <p>{this.state.error}</p>
            </div>
          ) : (
            <Editor
              theme="vs-dark"
              options={options}
              height="94vh"
              defaultLanguage="javascript"
              value={this.state.jsCode}
            />
          )}
        </div>
      </>
    );
  }
}

const container = document.getElementById("root");
const root = createRoot(container);
root.render(<App />);
