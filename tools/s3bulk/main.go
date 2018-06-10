package main

import (
	"bytes"
	"flag"
	"fmt"
	"os/exec"

	"github.com/aws/aws-sdk-go/aws"
	//  "github.com/aws/aws-sdk-go/aws/awserr"
	"encoding/json"
	"os"
	"path"
	"path/filepath"
	"strconv"
	"strings"
	"sync"

	"github.com/aws/aws-sdk-go/aws/awsutil"
	"github.com/aws/aws-sdk-go/aws/credentials"
	"github.com/aws/aws-sdk-go/aws/session"
	"github.com/aws/aws-sdk-go/service/s3"
)

// definition of a worker instance
type Worker struct {
	Acl          string          // s3 acl for uploaded files - for our use either "public" or "private"
	Bucket       string          // s3 bucket to upload to
	Subfolder    string          // s3 subfolder destination (if needed)
	Svc          *s3.S3          // instance of s3 svc
	File_channel chan string     // the channel to get file names from (upload todo list)
	Wg           *sync.WaitGroup // wait group - to signal when worker is finished
	SourceDir    string          // where source files are to be uploaded
	DestDir      string          // where to move uploaded files to (on local box)
	Id           int             // worker id number for debugging
}

type PkgInfo struct {
	Origin  string `json:"origin"`
	Name    string `json:"name"`
	Version string `json:"version"`
	Release string `json:"release"`
}

// worker to get all files inside a directory (recursively)
func get_file_list(searchDir string, file_channel chan string, num_workers int, wg *sync.WaitGroup) {
	defer wg.Done() // signal we are finished at end of function or return

	// sub function of how to recurse/walk the directory structure of searchDir
	_ = filepath.Walk(searchDir, func(path string, f os.FileInfo, err error) error {

		// check if it's a file/directory (we just want files)
		file, err := os.Open(path)
		if err != nil {
			return nil
		}

		defer file.Close() // close file handle on return

		fi, err := file.Stat()
		if fi.IsDir() {
			return nil
		}

		path = strings.Replace(path, searchDir, "", 1)
		file_channel <- path // add file to the work channel (queue)
		return nil
	})

	// add num_workers empty files on as termination signal to them
	for i := 0; i < num_workers; i++ {
		file_channel <- ""
	}
}

// upload function for workers
// uploads a given file to s3
func (worker *Worker) upload(file string) (string, error) {
	res, err := exec.Command("hab", "pkg", "info", "-j", fmt.Sprintf("%s%s", worker.SourceDir, file)).Output()

	if err != nil {
		panic(err)
	}

	var pkgInfo PkgInfo

	err = json.Unmarshal(res, &pkgInfo)
	if err != nil {
		panic(err)
	}

	// s3 destination file path
	var platform string

	if strings.Contains(file, "windows") {
		platform = "windows"
	} else {
		platform = "linux"
	}

	filename := strings.Split(file, "/")
	destfile := fmt.Sprintf("/%s/%s/%s/%s/x86_64/%s/%s", pkgInfo.Origin, pkgInfo.Name, pkgInfo.Version, pkgInfo.Release, platform, filename[len(filename)-1])
	worker.println("uploading to " + destfile)

	// open and read file
	f, err := os.Open(worker.SourceDir + file)

	if err != nil {
		return "Couldn't open file", err
	}

	defer f.Close()
	fileInfo, _ := f.Stat()
	var size = fileInfo.Size()
	buffer := make([]byte, size)
	f.Read(buffer)
	fileBytes := bytes.NewReader(buffer)

	params := &s3.PutObjectInput{
		Bucket: aws.String(worker.Bucket),
		Key:    aws.String(destfile),
		Body:   fileBytes,
		ACL:    aws.String(worker.Acl),
	}


	// try the actual s3 upload
	resp, err := worker.Svc.PutObject(params)
	if err != nil {
		return "", err
	} else {
		return awsutil.StringValue(resp), nil
	}
}

// doUploads function for workers
//
// reads from the file channel (queue),
// calls upload function for each,
// then moves uploaded files to worker.DestDir
func (worker *Worker) doUploads() {

	defer worker.Wg.Done() // notify parent when I complete

	worker.println("doUploads() started")

	// loop until I receive "" as a termination signal
	for {
		file := <-worker.File_channel
		if file == "" {
			break
		}
		worker.println("File to upload: " + file)
		response, err := worker.upload(file)
		if err != nil {
			worker.println("error uploading" + file + ": " + response + " " + err.Error())
		} else {
			worker.println(response)
			// make destination directory if needed
			filename := path.Base(file)
			directory := strings.Replace(file, "/"+filename, "", 1)
			os.MkdirAll(worker.DestDir+directory, 0775)
			// move file
			os.Rename(worker.SourceDir+file, worker.DestDir+file)
		}
	}
	worker.println("doUploads() finished")
}

// function to print out messages prefixed with worker-[id]
func (worker *Worker) println(message string) {
	fmt.Println("Worker-" + strconv.Itoa(worker.Id) + ": " + message)
}

func main() {

	bucketFlag := flag.String("bucket", "my-s3-bucket", "s3 bucket to upload to")
	subfolderFlag := flag.String("subfolder", "", "subfolder in s3 bucket, can be blank")
	num_workersFlag := flag.Int("workers", 100, "number of upload workers to use")
	regionFlag := flag.String("region", "eu-west-1", "aws region")
	aclFlag := flag.String("acl", "private", "s3 upload acl - use either private or public")
	sourceDirFlag := flag.String("sourcedir", "files/", "source directory")
	destDirFlag := flag.String("destdir", "files-uploaded/", "dest dir for uploaded files (on local box)")
    endpointFlag := flag.String("endpoint", "http://localhost:9000", "S3 URI in case of local S3")
    backendTypeFlag := flag.String("backend", "minio", "S3 URI in case of local S3")

	flag.Parse()

	bucket := *bucketFlag
	subfolder := *subfolderFlag
	num_workers := *num_workersFlag
	region := *regionFlag
	acl := *aclFlag
	sourceDir := *sourceDirFlag
	destDir := *destDirFlag
    endpoint := *endpointFlag
    backend := *backendTypeFlag

	fmt.Println("Using options:")
	fmt.Println("bucket:", bucket)
	fmt.Println("subfolder:", subfolder)
	fmt.Println("num_workers:", num_workers)
	fmt.Println("region:", region)
	fmt.Println("acl:", acl)
	fmt.Println("sourceDir:", sourceDir)
	fmt.Println("destDir:", destDir)

	var wg sync.WaitGroup
	wg.Add(num_workers + 1) // add 1 to account for the get_file_list thread!

	// file channel and thread to get the files
	file_channel := make(chan string, 0)
	go get_file_list(sourceDir, file_channel, num_workers, &wg)

	// set up s3 credentials from environment variables
	// these are shared to every worker
	creds := credentials.NewEnvCredentials()

	fmt.Println("Starting " + strconv.Itoa(num_workers) + " workers...")

	// create the desired number of workers
	for i := 1; i <= num_workers; i++ {
		// make a new worker
        awsConfig := &aws.Config{Region: aws.String(region), Credentials: creds, LogLevel: aws.LogLevel(1), S3ForcePathStyle: aws.Bool(true)}
        if backend == "minio" {
            awsConfig.Endpoint = aws.String(endpoint)
        }
        sess := session.New(awsConfig)
		svc := s3.New(sess)
		worker := &Worker{Acl: acl, Bucket: bucket, Subfolder: subfolder, Svc: svc, File_channel: file_channel, Wg: &wg, SourceDir: sourceDir, DestDir: destDir, Id: i}
		go worker.doUploads()
	}

	// wait for all workers to finish
	// (1x file worker and all uploader workers)
	wg.Wait()
}
