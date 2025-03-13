package main

// odr-webapi - a simple API for updating DLS text files
// This API receives text via POST requests and writes it to a file

import (
	"fmt"
	"io"
	"log"
	"net/http"
	"os"
	"path/filepath"
	"time"

	"github.com/spf13/pflag"
)

// Version information
const (
	AppName    = "odr-webapi"
	AppVersion = "0.0.3"
)

// Config holds the application configuration
type Config struct {
	Port       int
	TargetPath string
	AuthToken  string
}

// getLastUpdateTime returns the last modification time of the specified file
func getLastUpdateTime(filePath string) string {
	fileInfo, err := os.Stat(filePath)
	if err != nil {
		if os.IsNotExist(err) {
			return "Never"
		}
		log.Printf("Error getting file info: %v", err)
		return "Unknown"
	}
	return fileInfo.ModTime().Format(time.RFC3339)
}

// tokenAuthMiddleware validates the authentication token for protected endpoints
func tokenAuthMiddleware(next http.HandlerFunc, config *Config) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		// Token is required for POST requests to /api/dls
		if r.URL.Path == "/api/dls" && r.Method == http.MethodPost {
			// Check the Authorization header
			token := r.Header.Get("Authorization")

			// If no token is provided or it doesn't match, return 401 Unauthorized
			if token != config.AuthToken {
				http.Error(w, "Unauthorized: Invalid or missing token", http.StatusUnauthorized)
				log.Printf("Unauthorized access attempt from %s", r.RemoteAddr)
				return
			}
		}

		// If token is valid or not required, proceed to the next handler
		next(w, r)
	}
}

func main() {
	// Initialize configuration
	var config Config

	// Define command-line flags
	pflag.IntP("port", "p", 0, "Port on which the API should run (mandatory)")
	pflag.StringP("target", "t", "", "Full path to DLS text file (mandatory)")
	pflag.StringP("token", "k", "", "Authentication token for API access (mandatory for POST requests)")

	// Define custom usage information
	pflag.Usage = func() {
		fmt.Printf("%s v%s - a simple API for updating DLS text files\n", AppName, AppVersion)
		fmt.Println("\nUsage:")
		fmt.Printf("  %s --port PORT --target TARGET_PATH --token AUTH_TOKEN\n", AppName)
		fmt.Println("\nOptions:")
		fmt.Println("  --port, -p PORT")
		fmt.Println("        Port on which the API should run")
		fmt.Println("  --target, -t TARGET_PATH")
		fmt.Println("        Full path to DLS text file")
		fmt.Println("  --token, -k AUTH_TOKEN")
		fmt.Println("        Authentication token for API access (required for POST requests)")
		fmt.Println("\nEndpoints:")
		fmt.Println("  POST /api/dls")
		fmt.Println("        Updates the DLS text file with the request body")
		fmt.Println("        Requires Authorization header with token")
		fmt.Println("  GET  /api/status")
		fmt.Println("        Returns the API status and last update time")
		fmt.Println("\nExamples:")
		fmt.Printf("  %s --port=9000 --target=/dabplus/dls/dls.txt --token=mySecretToken\n", AppName)
		fmt.Printf("  %s --port 9000 --target /dabplus/dls/dls.txt --token mySecretToken\n", AppName)
		fmt.Println("\nAuthentication:")
		fmt.Println("  All POST requests must include the token in the Authorization header:")
		fmt.Println("  curl -X POST -H \"Authorization: mySecretToken\" --data \"New DLS text\" http://localhost:9000/api/dls")
	}

	pflag.Parse()

	// Extract values from parsed flags
	port, _ := pflag.CommandLine.GetInt("port")
	target, _ := pflag.CommandLine.GetString("target")
	token, _ := pflag.CommandLine.GetString("token")

	// Validate required parameters
	if port <= 0 || target == "" || token == "" {
		fmt.Println("Error: All parameters (port, target, token) are mandatory")
		pflag.Usage()
		os.Exit(1)
	}

	// Populate configuration
	config.Port = port
	config.TargetPath = target
	config.AuthToken = token

	// Validate target directory existence
	targetDir := filepath.Dir(config.TargetPath)
	if _, err := os.Stat(targetDir); os.IsNotExist(err) {
		log.Fatalf("Target directory does not exist: %s", targetDir)
	}

	// Set up API endpoints
	setupEndpoints(&config)

	// Start the server
	serverAddr := fmt.Sprintf(":%d", config.Port)
	log.Printf("%s v%s starting on http://localhost%s", AppName, AppVersion, serverAddr)
	log.Printf("DLS text will be written to: %s", config.TargetPath)
	log.Fatal(http.ListenAndServe(serverAddr, nil))
}

// setupEndpoints configures all API endpoints
func setupEndpoints(config *Config) {
	// Handler for writing text data
	http.HandleFunc("/api/dls", tokenAuthMiddleware(func(w http.ResponseWriter, r *http.Request) {
		// Only allow POST method
		if r.Method != http.MethodPost {
			http.Error(w, "Only POST method is supported", http.StatusMethodNotAllowed)
			return
		}

		// Read the request content
		body, err := io.ReadAll(r.Body)
		if err != nil {
			http.Error(w, "Error reading request body", http.StatusBadRequest)
			log.Printf("Error reading request body: %v", err)
			return
		}
		defer r.Body.Close()

		// Write the text to the target file
		err = os.WriteFile(config.TargetPath, body, 0644)
		if err != nil {
			http.Error(w, "Error writing to file", http.StatusInternalServerError)
			log.Printf("Error writing to %s: %v", config.TargetPath, err)
			return
		}

		// Get the updated time directly from the file
		lastUpdateTime := getLastUpdateTime(config.TargetPath)

		// Confirm success
		w.WriteHeader(http.StatusOK)
		w.Header().Set("Content-Type", "application/json")
		fmt.Fprintf(w, `{"status":"success","message":"Text successfully updated","lastUpdate":"%s"}`, lastUpdateTime)
		log.Printf("Text successfully written to %s at %s", config.TargetPath, lastUpdateTime)
	}, config))

	// Status endpoint to check if the API is running (no authentication required)
	http.HandleFunc("/api/status", func(w http.ResponseWriter, r *http.Request) {
		lastUpdateTime := getLastUpdateTime(config.TargetPath)
		w.Header().Set("Content-Type", "application/json")
		fmt.Fprintf(w, `{"status":"online","target":"%s","lastUpdate":"%s"}`,
			config.TargetPath, lastUpdateTime)
	})
}
